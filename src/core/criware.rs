use std::{
    os::raw::c_char,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex, Once,
    },
};
use fnv::FnvHashMap;
use once_cell::sync::Lazy;
use regex::Regex;
use crate::{
    core::Hachimi,
    il2cpp::{symbols, types::*, ext::{Il2CppObjectExt, StringExt}},
};

#[repr(C)]
pub struct CueInfo {
    pub id: i32,
    pub type_: i32,
    pub name: *mut c_char,
}

#[derive(Clone)]
struct CaptionData {
    text: String,
    cue_sheet: String,
    cue_id: i32,
    character_id: i32,
    voice_id: i32,
}

static ACB_CAPTIONS: Lazy<Mutex<FnvHashMap<usize, CaptionData>>> = Lazy::new(|| Mutex::default());
static PLAYER_ACB: Lazy<Mutex<FnvHashMap<usize, usize>>> = Lazy::new(|| Mutex::default());
static ACTIVE_PLAYERS: Lazy<Mutex<FnvHashMap<usize, usize>>> = Lazy::new(|| Mutex::default());
static CAPTION_REQUESTS: Lazy<Mutex<Vec<CaptionData>>> = Lazy::new(|| Mutex::default());

static CUE_SHEET_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"_(?:9)*(\d{4})(?:\d{2})*_(\d{4})*_*(\d{2})*(?:\d{2})*$").unwrap());

// ============================================================================
// Caption filter rules
// Edit these constants to tune which cues show captions.
//
// Rules are organised into groups. Within each group, suppression rules are
// listed first, followed immediately by any exception rules that can override
// them. Constants with no exceptions are noted as such.
// ============================================================================

// ----------------------------------------------------------------------------
// Group 1 — Cue name patterns
// Unconditional — no exceptions apply.
// Any cue whose name contains one of these substrings is always suppressed.
// ----------------------------------------------------------------------------

const SUPPRESS_CUE_NAME_PATTERNS: &[&str] = &[
    "snd_voi_story",
    "story_",
    "chara_story",
    "arc_story",
    "snd_voi_evt",
    "evt_",
    "_gallery_",
    "_home_",
    "_tc_",
    "_title_",
    "_gacha_",
    "_factorresearch_"
];

// ----------------------------------------------------------------------------
// Group 2 — Voice ID blocklist
// Unconditional — no exceptions apply.
// Any cue whose voice_id appears in this list is always suppressed.
// ----------------------------------------------------------------------------

const SUPPRESS_VOICE_IDS: &[i32] = &[
    95001,
];

// ----------------------------------------------------------------------------
// Group 3 — NPC / system character range
// Suppresses cues from characters at or above the chara_id threshold
// (generic system voices, NPCs, etc.).
// Exception: specific voice IDs listed below are allowed through regardless.
// ----------------------------------------------------------------------------

/// chara_ids at or above this value are treated as NPC / system characters.
const SUPPRESS_NPC_CHARA_ID_THRESHOLD: i32 = 9000;

/// Voice IDs exempt from the NPC chara_id suppression above.
const EXCEPT_NPC_ALLOW_VOICE_IDS: &[i32] = &[
    95005,
    95006,
    70000,
];

// ----------------------------------------------------------------------------
// Group 4 — Training scene filter
// Applies only to cues whose name contains "_training_".
// Suppresses low-index MasterCharacterSystemText entry IDs (generic ambient
// lines) and specific extra IDs. Several exception rules can override this.
// ----------------------------------------------------------------------------

/// Entry IDs below this threshold are suppressed in training scenes.
const SUPPRESS_TRAINING_CUE_ID_BELOW: i32 = 29;

/// Additional entry IDs suppressed in training scenes even when at or above
/// the threshold above.
const SUPPRESS_TRAINING_CUE_ID_EXTRA: &[i32] = &[
    39,
];

/// Entry IDs that are always shown in training scenes regardless of the
/// threshold.
const EXCEPT_TRAINING_ALLOW_CUE_IDS: &[i32] = &[
    8,
    9,
    12,
    13,
];

/// Inclusive voice ID ranges always shown in training scenes regardless of
/// the entry ID threshold (e.g. support card voices).
const EXCEPT_TRAINING_ALLOW_VOICE_ID_RANGES: &[(i32, i32)] = &[
    (2030, 2037), // support card voices
];

/// Voice IDs at or above any value here are always shown in training scenes
/// (e.g. special NPC voices above a known threshold).
const EXCEPT_TRAINING_ALLOW_VOICE_ID_MIN: &[i32] = &[
    93000,
];

/// Voice IDs that require a view ID check before being shown in training
/// scenes. Shown only when the current view ID is in the list below.
const EXCEPT_TRAINING_SCENE_CHECK_VOICE_IDS: &[i32] = &[
    20025,
];

/// View IDs that allow the voice IDs above through in training scenes.
/// Any other view causes those cues to be suppressed.
const EXCEPT_TRAINING_SCENE_ALLOWED_VIEW_IDS: &[i32] = &[
    5901,
];

// ----------------------------------------------------------------------------
// Group 5 — View ID filter
// Suppresses or force-shows captions based on the current SceneManager
// view ID, independently of all other rules.
// Force-show takes priority: if the view is in the allow list, all other
// suppression rules (including the suppression list below) are bypassed.
// ----------------------------------------------------------------------------

/// View IDs where captions are always suppressed, regardless of cue name,
/// voice ID, or character ID.
const SUPPRESS_VIEW_IDS: &[i32] = &[
    101,  // Home
    3200, // Umamusume Stories
];

/// View IDs where captions are always shown, overriding every other
/// suppression rule — including SUPPRESS_VIEW_IDS above.
const EXCEPT_VIEW_IDS: &[i32] = &[
    5212, // Archive — Voices
];

static INIT: Once = Once::new();

// --- #7 fix: replace `static mut usize` with AtomicUsize so hook callbacks
// on audio threads can read the trampoline addresses without data races. ---
// Ordering::Relaxed is sufficient: the Once in init() provides the
// happens-before edge that makes the stored values visible to all threads
// after init() returns, and the values are never mutated again after that.

type GetCueInfoByIdFn = extern "C" fn(acb: usize, id: i32, info: *mut CueInfo) -> bool;
static GETCUEINFOBYID_ORIG: AtomicUsize = AtomicUsize::new(0);
extern "C" fn get_cue_info_by_id(acb: usize, id: i32, info: *mut CueInfo) -> bool {
    let addr = GETCUEINFOBYID_ORIG.load(Ordering::Relaxed);
    if addr == 0 { return false; }
    let orig: GetCueInfoByIdFn = unsafe { std::mem::transmute(addr) };
    let res = orig(acb, id, info);
    if res && !info.is_null() && !unsafe { (*info).name }.is_null() {
        process_cue_info(acb, info);
    }
    res
}

type GetCueInfoByNameFn = extern "C" fn(acb: usize, name: *const c_char, info: *mut CueInfo) -> bool;
static GETCUEINFOBYNAME_ORIG: AtomicUsize = AtomicUsize::new(0);
extern "C" fn get_cue_info_by_name(acb: usize, name: *const c_char, info: *mut CueInfo) -> bool {
    let addr = GETCUEINFOBYNAME_ORIG.load(Ordering::Relaxed);
    if addr == 0 { return false; }
    let orig: GetCueInfoByNameFn = unsafe { std::mem::transmute(addr) };
    let res = orig(acb, name, info);
    if res && !info.is_null() && !unsafe { (*info).name }.is_null() {
        process_cue_info(acb, info);
    }
    res
}

// --- #10 fix: helper that locks a mutex and silently skips on poison instead
// of unwrap()-panicking.  A poisoned mutex means a previous hook invocation
// panicked while holding it; the safest recovery is to skip this call rather
// than abort the process. ---
macro_rules! lock_or_return {
    ($mutex:expr) => {
        match $mutex.lock() {
            Ok(g) => g,
            Err(e) => {
                warn!("[criware] mutex poisoned, skipping: {}", e);
                return;
            }
        }
    };
    ($mutex:expr, $ret:expr) => {
        match $mutex.lock() {
            Ok(g) => g,
            Err(e) => {
                warn!("[criware] mutex poisoned, skipping: {}", e);
                return $ret;
            }
        }
    };
}

fn process_cue_info(acb: usize, info: *mut CueInfo) {
    // Always clear the stale entry first, regardless of whether captions are enabled,
    // so we don't fire old data if captions are toggled back on later.
    // --- #10 fix: use lock_or_return! instead of unwrap() ---
    lock_or_return!(ACB_CAPTIONS).remove(&acb);

    let config = Hachimi::instance().config.load();
    if !config.caption.caption_enable { return; }
    let do_log = config.debug_mode;

    let cue_name = unsafe {
        // --- #8 fix: CStr::from_ptr walks until \0 with no length bound.
        // Scan manually up to a safe ceiling so a non-terminated CriWare
        // string can't cause an out-of-bounds read. ---
        const MAX_CUE_NAME_LEN: usize = 512;
        let ptr = (*info).name as *const u8;
        let len = (0..MAX_CUE_NAME_LEN).find(|&i| *ptr.add(i) == 0).unwrap_or(MAX_CUE_NAME_LEN);
        String::from_utf8_lossy(std::slice::from_raw_parts(ptr, len)).into_owned()
    };
    let cue_id = unsafe { (*info).id };

    if let Some(caps) = CUE_SHEET_REGEX.captures(&cue_name) {
        let mut chara_id_str = caps.get(1).map_or("", |m| m.as_str());
        if let (Some(m2), Some(m3)) = (caps.get(2), caps.get(3)) {
            if m3.as_str() == "01" { chara_id_str = m2.as_str(); }
        }
        if let Ok(chara_id) = chara_id_str.parse::<i32>() {
            let image = match symbols::get_assembly_image(c"umamusume.dll") {
                Ok(i) => i, Err(_) => return
            };
            let master_class = match symbols::get_class(image, c"Gallop", c"MasterCharacterSystemText") {
                Ok(c) => c, Err(_) => return
            };
            let get_by_chara_id_addr = symbols::get_method_addr_cached(master_class, c"GetByCharaId", 1);
            if get_by_chara_id_addr == 0 { return; }
            let get_by_chara_id: extern "C" fn(i32) -> *mut Il2CppObject =
                unsafe { std::mem::transmute(get_by_chara_id_addr) };

            let list = get_by_chara_id(chara_id);
            if list.is_null() { return; }

            if let Some(ilist) = crate::il2cpp::symbols::IList::<*mut crate::il2cpp::types::Il2CppObject>::new(list) {
                for item in ilist.iter() {
                    if item.is_null() { continue; }
                    let item_klass = unsafe { (*item).klass() };
                    let cue_id_field    = symbols::get_field_from_name(item_klass, c"CueId");
                    let cue_sheet_field = symbols::get_field_from_name(item_klass, c"CueSheet");
                    if cue_id_field.is_null() || cue_sheet_field.is_null() { continue; }

                    let item_cue_id = symbols::get_field_value::<i32>(item, cue_id_field);
                    let item_cue_sheet_ptr = symbols::get_field_object_value::<Il2CppString>(item, cue_sheet_field);

                    if item_cue_id != cue_id || item_cue_sheet_ptr.is_null() { continue; }

                    use crate::il2cpp::ext::Il2CppStringExt;
                    let item_cue_sheet = unsafe { (*item_cue_sheet_ptr).as_utf16str().to_string() };
                    if !cue_name.starts_with(&item_cue_sheet) { continue; }

                    let text_field     = symbols::get_field_from_name(item_klass, c"Text");
                    let voice_id_field = symbols::get_field_from_name(item_klass, c"VoiceId");
                    if text_field.is_null() || voice_id_field.is_null() { break; }

                    let text_ptr = symbols::get_field_object_value::<Il2CppString>(item, text_field);
                    let voice_id = symbols::get_field_value::<i32>(item, voice_id_field);

                    if text_ptr.is_null() { break; }

                    let orig_text  = unsafe { (*text_ptr).as_utf16str().to_string() };
                    let clean_text = orig_text.replace("\n\n", " ").replace("\n", " ");

                    // ── Fetch view ID once — used by rules 4 and 5 ──────────
                    let current_view_id: i32 = (|| -> Option<i32> {
                        let sm_class = symbols::get_class(image, c"Gallop", c"SceneManager").ok()?;
                        let singleton = symbols::SingletonLike::new(sm_class)?;
                        let sm = singleton.instance();
                        if sm.is_null() { return None; }
                        let get_view_id_addr = symbols::get_method_addr_cached(sm_class, c"GetCurrentViewId", 0);
                        if get_view_id_addr == 0 { return None; }
                        let get_view_id: extern "C" fn(*mut Il2CppObject) -> i32 =
                            unsafe { std::mem::transmute(get_view_id_addr) };
                        Some(get_view_id(sm))
                    })().unwrap_or(0);

                    // ── Rule 1 — Cue name pattern ────────────────────────────
                    // Suppression: cue name contains a blocked substring.
                    // No exception.
                    let suppressed_pattern = SUPPRESS_CUE_NAME_PATTERNS
                        .iter().find(|&&p| cue_name.contains(p)).copied();

                    // ── Rule 2 — Voice ID blocklist ──────────────────────────
                    // Suppression: voice_id is in the blocked list.
                    // No exception.
                    let suppressed_voice = SUPPRESS_VOICE_IDS.contains(&voice_id);

                    // ── Rule 3 — NPC / system character range ────────────────
                    // Suppression: chara_id >= threshold.
                    // Exception:   voice_id is in EXCEPT_NPC_ALLOW_VOICE_IDS.
                    let suppressed_npc = chara_id >= SUPPRESS_NPC_CHARA_ID_THRESHOLD
                        && !EXCEPT_NPC_ALLOW_VOICE_IDS.contains(&voice_id);

                    // ── Rule 4 — Training scene filter ───────────────────────
                    // Suppression: cue name contains "_training_" and entry ID
                    //              is below threshold or in the extra list.
                    // Exception A: entry ID is in EXCEPT_TRAINING_ALLOW_CUE_IDS.
                    // Exception B: voice_id is in an allowed range.
                    // Exception C: voice_id >= a minimum allowed value.
                    // Exception D: voice_id is in EXCEPT_TRAINING_SCENE_CHECK_VOICE_IDS
                    //              and the current view ID is in EXCEPT_TRAINING_SCENE_ALLOWED_VIEW_IDS.
                    let mut suppressed_training = false;
                    let mut suppressed_training_reason = "";
                    if cue_name.contains("_training_")
                        && (item_cue_id < SUPPRESS_TRAINING_CUE_ID_BELOW
                            || SUPPRESS_TRAINING_CUE_ID_EXTRA.contains(&item_cue_id))
                    {
                        // Exception A
                        let exc_a = EXCEPT_TRAINING_ALLOW_CUE_IDS.contains(&item_cue_id);
                        // Exception B
                        let exc_b = EXCEPT_TRAINING_ALLOW_VOICE_ID_RANGES
                            .iter().any(|&(lo, hi)| voice_id >= lo && voice_id <= hi);
                        // Exception C
                        let exc_c = EXCEPT_TRAINING_ALLOW_VOICE_ID_MIN
                            .iter().any(|&min| voice_id >= min);
                        // Exception D
                        let exc_d = EXCEPT_TRAINING_SCENE_CHECK_VOICE_IDS.contains(&voice_id)
                            && EXCEPT_TRAINING_SCENE_ALLOWED_VIEW_IDS.contains(&current_view_id);

                        if !exc_a && !exc_b && !exc_c && !exc_d {
                            suppressed_training = true;
                            suppressed_training_reason = if EXCEPT_TRAINING_SCENE_CHECK_VOICE_IDS.contains(&voice_id) {
                                "training scene: view ID not in allowed list"
                            } else if SUPPRESS_TRAINING_CUE_ID_EXTRA.contains(&item_cue_id) {
                                "training scene: entry ID in extra suppress list"
                            } else {
                                "training scene: entry ID below suppress threshold"
                            };
                        }
                    }

                    // ── Rule 5 — Global view ID suppression ──────────────────
                    // Suppression: current view ID is in SUPPRESS_VIEW_IDS.
                    // No exception.
                    let suppressed_view = !SUPPRESS_VIEW_IDS.is_empty()
                        && SUPPRESS_VIEW_IDS.contains(&current_view_id);

                    // ── Rule 6 — Global view ID force-show ───────────────────
                    // Exception: current view ID is in EXCEPT_VIEW_IDS.
                    // Overrides all suppression rules above.
                    let force_show_view = !EXCEPT_VIEW_IDS.is_empty()
                        && EXCEPT_VIEW_IDS.contains(&current_view_id);

                    // ── Final decision ───────────────────────────────────────
                    let show = force_show_view
                        || (suppressed_pattern.is_none()
                            && !suppressed_voice
                            && !suppressed_npc
                            && !suppressed_training
                            && !suppressed_view);

                    if do_log {
                        if show {
                            info!(
                                "[captions] SHOW | chara_id={} voice_id={} cue_id={} item_cue_id={} view_id={} cue_name={}{}",
                                chara_id, voice_id, cue_id, item_cue_id, current_view_id, cue_name,
                                if force_show_view { " [force-show view]" } else { "" }
                            );
                        } else {
                            let reason = if let Some(p) = suppressed_pattern {
                                format!("cue_name pattern \"{}\"", p)
                            } else if suppressed_voice {
                                format!("suppressed voice_id {}", voice_id)
                            } else if suppressed_npc {
                                format!("NPC chara_id >= {} (voice_id={})", SUPPRESS_NPC_CHARA_ID_THRESHOLD, voice_id)
                            } else if suppressed_view {
                                format!("suppressed view_id {}", current_view_id)
                            } else {
                                suppressed_training_reason.to_owned()
                            };
                            info!(
                                "[captions] SKIP  | chara_id={} voice_id={} cue_id={} item_cue_id={} view_id={} cue_name={} reason={}",
                                chara_id, voice_id, cue_id, item_cue_id, current_view_id, cue_name, reason
                            );
                        }
                    }

                    if show {
                        // --- #10 fix ---
                        lock_or_return!(ACB_CAPTIONS).insert(acb, CaptionData {
                            text: clean_text,
                            cue_sheet: item_cue_sheet,
                            cue_id: item_cue_id,
                            character_id: chara_id,
                            voice_id,
                        });
                    }
                    break;
                }
            }
        }
    }
}

type SetCueIdFn = extern "C" fn(player: usize, acb: usize, id: i32);
static SETCUEID_ORIG: AtomicUsize = AtomicUsize::new(0);
extern "C" fn set_cue_id(player: usize, acb: usize, id: i32) {
    let addr = SETCUEID_ORIG.load(Ordering::Relaxed);
    if addr == 0 { return; }
    let orig: SetCueIdFn = unsafe { std::mem::transmute(addr) };
    // --- #10 fix ---
    if lock_or_return!(ACB_CAPTIONS, ()).contains_key(&acb) {
        lock_or_return!(PLAYER_ACB, ()).insert(player, acb);
    }
    orig(player, acb, id)
}

type StartFn = extern "C" fn(player: usize) -> u32;
static START_ORIG: AtomicUsize = AtomicUsize::new(0);

fn process_caption_requests() {
    // --- #10 fix ---
    let mut requests = lock_or_return!(CAPTION_REQUESTS);
    // Re-check caption_enable at display time; the user may have toggled it off
    // between when the cue was queued and when it actually starts playing.
    if !Hachimi::instance().config.load().caption.caption_enable {
        requests.clear();
        return;
    }
    for caption_data in requests.drain(..) {
        let length = (|| -> Option<f32> {
            let image = symbols::get_assembly_image(c"umamusume.dll").ok()?;
            let audio_manager_class = symbols::get_class(image, c"Gallop", c"AudioManager").ok()?;
            let audio_manager = symbols::SingletonLike::new(audio_manager_class)?.instance();
            if audio_manager.is_null() { return None; }
            if !crate::il2cpp::hook::UnityEngine_CoreModule::Object::IsNativeObjectAlive(audio_manager) { return None; }

            let get_cue_length_method = symbols::get_method_cached(audio_manager_class, c"GetCueLength", 2).ok()?;
            let mut exc = std::ptr::null_mut();
            // --- #9 fix: to_il2cpp_string() can return null (OOM / missing
            // API); passing null as a managed String param throws a managed
            // NullReferenceException that becomes an unhandled native crash
            // on Android.  Bail out early so we fall back to the 3.0s default. ---
            let cue_sheet_il2 = caption_data.cue_sheet.to_il2cpp_string();
            if cue_sheet_il2.is_null() { return None; }
            let mut params = [
                cue_sheet_il2 as *mut std::ffi::c_void,
                &caption_data.cue_id as *const _ as *mut std::ffi::c_void
            ];

            let res = crate::il2cpp::api::il2cpp_runtime_invoke(
                get_cue_length_method,
                audio_manager as *mut std::ffi::c_void,
                params.as_mut_ptr(),
                &mut exc
            );

            if !exc.is_null() || res.is_null() { return None; }
            Some(unsafe { *(crate::il2cpp::api::il2cpp_object_unbox(res) as *mut f32) })
        })().unwrap_or(3.0);

        let localized_text = Hachimi::instance().localized_data.load()
            .character_system_text_dict
            .get(&caption_data.character_id)
            .and_then(|dict| dict.get(&caption_data.voice_id))
            .cloned()
            .unwrap_or_else(|| caption_data.text.clone());

        crate::core::captions::Captions::init();
        crate::core::captions::Captions::set_display_time(length);

        let config = Hachimi::instance().config.load();
        crate::core::captions::Captions::set_format(
            config.caption.caption_font_size,
            &config.caption.caption_color,
            &config.caption.caption_outline_size,
            &config.caption.caption_outline_color,
            config.caption.caption_pos_x,
            config.caption.caption_pos_y,
            config.caption.caption_bg_alpha,
        );
        crate::core::captions::Captions::show(&localized_text);
    }
}

extern "C" fn start(player: usize) -> u32 {
    let addr = START_ORIG.load(Ordering::Relaxed);
    if addr == 0 { return 0; }
    let orig: StartFn = unsafe { std::mem::transmute(addr) };

    // --- #10 fix ---
    let acb = lock_or_return!(PLAYER_ACB, orig(player)).get(&player).cloned();

    if let Some(acb) = acb {
        // --- #10 fix ---
        let caption_data = lock_or_return!(ACB_CAPTIONS, orig(player)).get(&acb).cloned();

        if let Some(caption_data) = caption_data {
            // --- #10 fix ---
            lock_or_return!(ACTIVE_PLAYERS, orig(player)).insert(player, acb);
            lock_or_return!(CAPTION_REQUESTS, orig(player)).push(caption_data);

            // --- #19 fix: Thread::main_thread() calls .expect() internally;
            // guard against an empty thread list by checking attached_threads
            // before calling the convenience method. ---
            let threads = symbols::Thread::attached_threads();
            if let Some(main) = threads.first() {
                main.schedule(process_caption_requests);
            } else {
                warn!("[criware] no attached threads, caption request dropped");
            }
        }
    }

    orig(player)
}

type StopFn = extern "C" fn(player: usize);
static STOP_ORIG: AtomicUsize = AtomicUsize::new(0);
extern "C" fn stop(player: usize) {
    let addr = STOP_ORIG.load(Ordering::Relaxed);
    if addr == 0 { return; }
    let orig: StopFn = unsafe { std::mem::transmute(addr) };
    orig(player);
    clear_active_player(player);
}

type StopWithoutReleaseTimeFn = extern "C" fn(player: usize);
static STOPWITHOUTRELEASETIME_ORIG: AtomicUsize = AtomicUsize::new(0);
extern "C" fn stop_without_release_time(player: usize) {
    let addr = STOPWITHOUTRELEASETIME_ORIG.load(Ordering::Relaxed);
    if addr == 0 { return; }
    let orig: StopWithoutReleaseTimeFn = unsafe { std::mem::transmute(addr) };
    orig(player);
    clear_active_player(player);
}

type PauseFn = extern "C" fn(player: usize, sw: bool);
static PAUSE_ORIG: AtomicUsize = AtomicUsize::new(0);
extern "C" fn pause(player: usize, sw: bool) {
    let addr = PAUSE_ORIG.load(Ordering::Relaxed);
    if addr == 0 { return; }
    let orig: PauseFn = unsafe { std::mem::transmute(addr) };
    orig(player, sw);
    if !sw {
        clear_active_player(player);
    }
}

type ReleaseFn = extern "C" fn(acb: usize);
static RELEASE_ORIG: AtomicUsize = AtomicUsize::new(0);
extern "C" fn release(acb: usize) {
    // --- #10 fix ---
    lock_or_return!(ACB_CAPTIONS).remove(&acb);
    let addr = RELEASE_ORIG.load(Ordering::Relaxed);
    if addr == 0 { return; }
    let orig: ReleaseFn = unsafe { std::mem::transmute(addr) };
    orig(acb);
}

fn clear_active_player(player: usize) {
    // --- #10 fix ---
    let acb = lock_or_return!(ACTIVE_PLAYERS).remove(&player);
    if let Some(acb) = acb {
        lock_or_return!(ACB_CAPTIONS).remove(&acb);
        crate::core::captions::Captions::cleanup();
    }
}

pub fn init(handle: usize) {
    INIT.call_once(|| {
        info!("Initializing criware hooks");
        let hachimi = Hachimi::instance();

        let get_cue_info_by_id_addr        = crate::core::utils::get_proc_address(handle, c"criAtomExAcb_GetCueInfoById");
        let get_cue_info_by_name_addr       = crate::core::utils::get_proc_address(handle, c"criAtomExAcb_GetCueInfoByName");
        let release_addr                    = crate::core::utils::get_proc_address(handle, c"criAtomExAcb_Release");
        let set_cue_id_addr                 = crate::core::utils::get_proc_address(handle, c"criAtomExPlayer_SetCueId");
        let start_addr                      = crate::core::utils::get_proc_address(handle, c"criAtomExPlayer_Start");
        let stop_addr                       = crate::core::utils::get_proc_address(handle, c"criAtomExPlayer_Stop");
        let stop_without_release_time_addr  = crate::core::utils::get_proc_address(handle, c"criAtomExPlayer_StopWithoutReleaseTime");
        let pause_addr                      = crate::core::utils::get_proc_address(handle, c"criAtomExPlayer_Pause");

        // --- CW-1 fix: log and skip individual hook failures instead of
        // panicking and aborting the entire CriWare init. ---
        if get_cue_info_by_id_addr != 0 {
            match hachimi.interceptor.hook(get_cue_info_by_id_addr, get_cue_info_by_id as *const () as usize) {
                Ok(tramp) => { GETCUEINFOBYID_ORIG.store(tramp, Ordering::Release); }
                Err(e) => { error!("[criware] failed to hook GetCueInfoById: {}", e); }
            }
        }
        if get_cue_info_by_name_addr != 0 {
            match hachimi.interceptor.hook(get_cue_info_by_name_addr, get_cue_info_by_name as *const () as usize) {
                Ok(tramp) => { GETCUEINFOBYNAME_ORIG.store(tramp, Ordering::Release); }
                Err(e) => { error!("[criware] failed to hook GetCueInfoByName: {}", e); }
            }
        }
        if release_addr != 0 {
            match hachimi.interceptor.hook(release_addr, release as *const () as usize) {
                Ok(tramp) => { RELEASE_ORIG.store(tramp, Ordering::Release); }
                Err(e) => { error!("[criware] failed to hook Release: {}", e); }
            }
        }
        if set_cue_id_addr != 0 {
            match hachimi.interceptor.hook(set_cue_id_addr, set_cue_id as *const () as usize) {
                Ok(tramp) => { SETCUEID_ORIG.store(tramp, Ordering::Release); }
                Err(e) => { error!("[criware] failed to hook SetCueId: {}", e); }
            }
        }
        if start_addr != 0 {
            match hachimi.interceptor.hook(start_addr, start as *const () as usize) {
                Ok(tramp) => { START_ORIG.store(tramp, Ordering::Release); }
                Err(e) => { error!("[criware] failed to hook Start: {}", e); }
            }
        }
        if stop_addr != 0 {
            match hachimi.interceptor.hook(stop_addr, stop as *const () as usize) {
                Ok(tramp) => { STOP_ORIG.store(tramp, Ordering::Release); }
                Err(e) => { error!("[criware] failed to hook Stop: {}", e); }
            }
        }
        if stop_without_release_time_addr != 0 {
            match hachimi.interceptor.hook(stop_without_release_time_addr, stop_without_release_time as *const () as usize) {
                Ok(tramp) => { STOPWITHOUTRELEASETIME_ORIG.store(tramp, Ordering::Release); }
                Err(e) => { error!("[criware] failed to hook StopWithoutReleaseTime: {}", e); }
            }
        }
        if pause_addr != 0 {
            match hachimi.interceptor.hook(pause_addr, pause as *const () as usize) {
                Ok(tramp) => { PAUSE_ORIG.store(tramp, Ordering::Release); }
                Err(e) => { error!("[criware] failed to hook Pause: {}", e); }
            }
        }
    });
}
