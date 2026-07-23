use crate::{
    core::{Hachimi, captions, live_utils::AudioPlayback},
    il2cpp::{
        ext::{Il2CppStringExt, StringExt, Il2CppObjectExt},
        symbols::{self, get_method_addr, get_field_from_name, SingletonLike, Array},
        types::*,
        api::*
    }
};

static mut CLASS: *mut Il2CppClass = 0 as _;
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

pub fn instance() -> *mut Il2CppObject {
    let Some(singleton) = SingletonLike::new(class()) else {
        return 0 as _;
    };
    singleton.instance()
}

static mut GET_CRIAUDIOMANAGER_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_CriAudioManager, GET_CRIAUDIOMANAGER_ADDR, *mut Il2CppObject,);

static mut _SONGPLAYBACK_FIELD: *mut FieldInfo = 0 as _;
pub fn get__songPlayback(this: *mut Il2CppObject) -> crate::core::live_utils::AudioPlayback {
    crate::il2cpp::symbols::get_field_value(this, unsafe { _SONGPLAYBACK_FIELD })
}
pub fn set__songPlayback(this: *mut Il2CppObject, value: crate::core::live_utils::AudioPlayback) {
    crate::il2cpp::symbols::set_field_value(this, unsafe { _SONGPLAYBACK_FIELD }, &value)
}

// Cached FieldInfo pointers for MasterCharacterSystemText items.
// All items in the list share the same class, so we resolve once on first use.
static mut CST_CUEID_FIELD: *mut FieldInfo = 0 as _;
static mut CST_CUESHEET_FIELD: *mut FieldInfo = 0 as _;
static mut CST_TEXT_FIELD: *mut FieldInfo = 0 as _;
static mut CST_VOICEID_FIELD: *mut FieldInfo = 0 as _;

static mut _SONGCHARAPLAYBACKS_FIELD: *mut FieldInfo = 0 as _;
pub fn get__songCharaPlaybacks(this: *mut Il2CppObject) -> *mut Il2CppArray {
    crate::il2cpp::symbols::get_field_object_value(this, unsafe { _SONGCHARAPLAYBACKS_FIELD })
}
pub fn set__songCharaPlaybacks(this: *mut Il2CppObject, value: *mut Il2CppArray) {
    crate::il2cpp::symbols::set_field_object_value(this, unsafe { _SONGCHARAPLAYBACKS_FIELD }, value)
}

static mut GET_CUE_LENGTH_ADDR: usize = 0;
impl_addr_wrapper_fn!(GetCueLength, GET_CUE_LENGTH_ADDR, f32, this: *mut Il2CppObject, cue_sheet: *mut Il2CppString, cue_id: i32);

// Cute.Cri.Audio RequestCueInfo
#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
pub struct RequestCueInfo {
    pub CueSheetName: *mut Il2CppString,
    pub CueName: *mut Il2CppString,
    pub CueId: i32,
}

// Cute.Cri SoundGroup
#[derive(Clone, Copy, PartialEq)]
#[repr(i32)]
pub enum SoundGroup {
    Bgm = 0,
    Se = 1,
    Voice = 2,
}

struct CstEntry {
    voice_id: i32,
    text: String,
}

// ── Caption filter constants ──────────────────────────────────────────────────
// These constants control which captions are shown by the PlayInternal hook.
// Rules are evaluated in order. Within each group, suppression rules appear
// first and any exception rules appear immediately after.

// Group 1 — Cue name patterns
// Any cue whose name contains one of these substrings is suppressed.
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
    "_kakao_",
    "_gacha_",
    "_factorresearch_",
];

// Group 2 — Voice ID blocklist
const SUPPRESS_VOICE_IDS: &[i32] = &[95001];

// Group 3 — NPC / system character range
const SUPPRESS_NPC_CHARA_ID_THRESHOLD: i32 = 9000;
const EXCEPT_NPC_ALLOW_VOICE_IDS: &[i32] = &[95005, 95006, 70000];

// Group 4 — Training scene filter
const SUPPRESS_TRAINING_CUE_ID_BELOW: i32 = 29;
const SUPPRESS_TRAINING_CUE_ID_EXTRA: &[i32] = &[39];
const EXCEPT_TRAINING_ALLOW_CUE_IDS: &[i32] = &[8, 9, 12, 13];
const EXCEPT_TRAINING_ALLOW_VOICE_ID_RANGES: &[(i32, i32)] = &[(2030, 2037)];
const EXCEPT_TRAINING_ALLOW_VOICE_ID_MIN: &[i32] = &[93000];
const EXCEPT_TRAINING_SCENE_CHECK_VOICE_IDS: &[i32] = &[20025];
const EXCEPT_TRAINING_SCENE_ALLOWED_VIEW_IDS: &[i32] = &[5901];

// Group 5 — View ID filter
const SUPPRESS_VIEW_IDS: &[i32] = &[
    101,  // Home
    3200, // Umamusume Stories
];
const EXCEPT_VIEW_IDS: &[i32] = &[
    5212, // Archive — Voices
];

/// Fetches the current SceneManager view ID via IL2CPP. Returns 0 on failure.
fn get_current_view_id() -> i32 {
    (|| -> Option<i32> {
        let image = symbols::get_assembly_image(c"umamusume.dll").ok()?;
        let sm_class = symbols::get_class(image, c"Gallop", c"SceneManager").ok()?;
        let sm = symbols::SingletonLike::new(sm_class)?.instance();
        if sm.is_null() { return None; }
        let addr = symbols::get_method_addr_cached(sm_class, c"GetCurrentViewId", 0);
        if addr == 0 { return None; }
        let get_view_id: extern "C" fn(*mut Il2CppObject) -> i32 =
            unsafe { std::mem::transmute(addr) };
        Some(get_view_id(sm))
    })().unwrap_or(0)
}

/// Looks up the MasterCharacterSystemText entry for the given character + cue,
/// applying all suppression/exception rules. Returns `None` if suppressed.
fn lookup_cst_entry(chara_id: i32, cue_id: i32, cue_name: &str) -> Option<CstEntry> {
    let do_log = captions::Captions::show_log_enabled();

    // ── Rule 1 — Cue name pattern (cheapest check, before any IL2CPP calls) ──
    if let Some(&pattern) = SUPPRESS_CUE_NAME_PATTERNS.iter().find(|&&p| cue_name.contains(p)) {
        if do_log {
            info!("[captions] SKIP  | cue_name={} reason=cue_name pattern \"{}\"",
                cue_name, pattern);
        }
        return None;
    }

    let image = symbols::get_assembly_image(c"umamusume.dll").ok()?;
    let master_class = symbols::get_class(image, c"Gallop", c"MasterCharacterSystemText").ok()?;
    let get_by_chara_id_addr = symbols::get_method_addr_cached(master_class, c"GetByCharaId", 1);
    if get_by_chara_id_addr == 0 { return None; }
    let get_by_chara_id: extern "C" fn(i32) -> *mut Il2CppObject =
        unsafe { std::mem::transmute(get_by_chara_id_addr) };

    let list = get_by_chara_id(chara_id);
    if list.is_null() { return None; }

    if let Some(ilist) = symbols::IList::<*mut Il2CppObject>::new(list) {
        for item in ilist.iter() {
            if item.is_null() { continue; }
            let item_klass = unsafe { (*item).klass() };

            // Resolve and cache FieldInfo pointers on first item.
            unsafe {
                if CST_CUEID_FIELD.is_null() {
                    CST_CUEID_FIELD    = symbols::get_field_from_name(item_klass, c"CueId");
                    CST_CUESHEET_FIELD = symbols::get_field_from_name(item_klass, c"CueSheet");
                    CST_TEXT_FIELD     = symbols::get_field_from_name(item_klass, c"Text");
                    CST_VOICEID_FIELD  = symbols::get_field_from_name(item_klass, c"VoiceId");
                }
            }
            let cue_id_field   = unsafe { CST_CUEID_FIELD };
            let cue_sheet_field = unsafe { CST_CUESHEET_FIELD };
            let text_field     = unsafe { CST_TEXT_FIELD };
            let voice_id_field = unsafe { CST_VOICEID_FIELD };

            if cue_id_field.is_null() || cue_sheet_field.is_null() { continue; }

            let item_cue_id = symbols::get_field_value::<i32>(item, cue_id_field);
            let item_cue_sheet_ptr = symbols::get_field_object_value::<Il2CppString>(item, cue_sheet_field);
            if item_cue_id != cue_id || item_cue_sheet_ptr.is_null() { continue; }

            let item_cue_sheet = unsafe { (*item_cue_sheet_ptr).as_utf16str().to_string() };
            if !cue_name.starts_with(&item_cue_sheet) { continue; }

            if text_field.is_null() || voice_id_field.is_null() { break; }

            let text_ptr = symbols::get_field_object_value::<Il2CppString>(item, text_field);
            let voice_id = symbols::get_field_value::<i32>(item, voice_id_field);
            if text_ptr.is_null() { break; }

            let orig_text = unsafe { (*text_ptr).as_utf16str().to_string() };
            let clean_text = orig_text.replace("\n\n", " ").replace("\n", " ");

            // ── Rule 2 — Voice ID blocklist ──────────────────────────────────
            if SUPPRESS_VOICE_IDS.contains(&voice_id) {
                if do_log {
                    info!("[captions] SKIP  | chara_id={} voice_id={} cue_id={} item_cue_id={} cue_name={} reason=suppressed voice_id {}",
                        chara_id, voice_id, cue_id, item_cue_id, cue_name, voice_id);
                }
                break;
            }

            // ── Rule 3 — NPC / system character range ────────────────────────
            if chara_id >= SUPPRESS_NPC_CHARA_ID_THRESHOLD
                && !EXCEPT_NPC_ALLOW_VOICE_IDS.contains(&voice_id)
            {
                if do_log {
                    info!("[captions] SKIP  | chara_id={} voice_id={} cue_id={} item_cue_id={} cue_name={} reason=NPC chara_id >= {} (voice_id={})",
                        chara_id, voice_id, cue_id, item_cue_id, cue_name,
                        SUPPRESS_NPC_CHARA_ID_THRESHOLD, voice_id);
                }
                break;
            }

            // ── Rules 4 & 5 require the current view ID ─────────────────────
            // Fetch it exactly once, only when we've passed rules 1–3.
            let current_view_id = get_current_view_id();

            // ── Rule 5 — Global view ID force-show ──────────────────────────
            let force_show_view = !EXCEPT_VIEW_IDS.is_empty()
                && EXCEPT_VIEW_IDS.contains(&current_view_id);

            // ── Rule 5 — Global view ID suppression ─────────────────────────
            let suppressed_view = !force_show_view
                && !SUPPRESS_VIEW_IDS.is_empty()
                && SUPPRESS_VIEW_IDS.contains(&current_view_id);
            if suppressed_view {
                if do_log {
                    info!("[captions] SKIP  | chara_id={} voice_id={} cue_id={} item_cue_id={} view_id={} cue_name={} reason=suppressed view_id {}",
                        chara_id, voice_id, cue_id, item_cue_id, current_view_id, cue_name, current_view_id);
                }
                break;
            }

            // ── Rule 4 — Training scene filter ──────────────────────────────
            let mut suppressed_training = false;
            let mut suppressed_training_reason = "";
            if !force_show_view
                && cue_name.contains("_training_")
                && (item_cue_id < SUPPRESS_TRAINING_CUE_ID_BELOW
                    || SUPPRESS_TRAINING_CUE_ID_EXTRA.contains(&item_cue_id))
            {
                let exc_a = EXCEPT_TRAINING_ALLOW_CUE_IDS.contains(&item_cue_id);
                let exc_b = EXCEPT_TRAINING_ALLOW_VOICE_ID_RANGES
                    .iter().any(|&(lo, hi)| voice_id >= lo && voice_id <= hi);
                let exc_c = EXCEPT_TRAINING_ALLOW_VOICE_ID_MIN
                    .iter().any(|&min| voice_id >= min);
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

            // ── Final decision ──────────────────────────────────────────────
            let show = force_show_view || !suppressed_training;

            if do_log {
                if show {
                    info!(
                        "[captions] SHOW | chara_id={} voice_id={} cue_id={} item_cue_id={} view_id={} cue_name={}{}",
                        chara_id, voice_id, cue_id, item_cue_id, current_view_id, cue_name,
                        if force_show_view { " [force-show view]" } else { "" }
                    );
                } else {
                    info!(
                        "[captions] SKIP  | chara_id={} voice_id={} cue_id={} item_cue_id={} view_id={} cue_name={} reason={}",
                        chara_id, voice_id, cue_id, item_cue_id, current_view_id, cue_name,
                        suppressed_training_reason
                    );
                }
            }

            if show {
                return Some(CstEntry {
                    voice_id,
                    text: clean_text,
                });
            }
            break;
        }
    }
    None
}

fn is_caption_redundant() -> bool {
    use std::sync::atomic::{AtomicUsize, Ordering};

    // Cached IL2CPP pointers — resolved once, stable for the process lifetime.
    static PARTS_CLASS: AtomicUsize = AtomicUsize::new(0);
    static OBJ_CLASS: AtomicUsize = AtomicUsize::new(0);
    static GO_CLASS: AtomicUsize = AtomicUsize::new(0);

    fn cached_class(cache: &AtomicUsize, asm: &std::ffi::CStr, ns: &std::ffi::CStr, name: &std::ffi::CStr) -> *mut Il2CppClass {
        let v = cache.load(Ordering::Relaxed);
        if v != 0 { return v as _; }
        let img = match symbols::get_assembly_image(asm) { Ok(i) => i, Err(_) => return std::ptr::null_mut() };
        let cls = match symbols::get_class(img, ns, name) { Ok(c) => c, Err(_) => return std::ptr::null_mut() };
        cache.store(cls as usize, Ordering::Relaxed);
        cls
    }

    let parts_class = cached_class(&PARTS_CLASS, c"umamusume.dll", c"Gallop", c"PartsCharaMessageBase");
    if parts_class.is_null() { return false; }

    let parts_type = il2cpp_type_get_object(il2cpp_class_get_type(parts_class)) as *mut Il2CppObject;
    if !parts_type.is_null() {
        let obj_class = cached_class(&OBJ_CLASS, c"UnityEngine.CoreModule.dll", c"UnityEngine", c"Object");
        if !obj_class.is_null() {
            let find_objects_addr = symbols::get_method_addr_cached(obj_class, c"FindObjectsOfType", 2);
            if find_objects_addr != 0 {
                let find_objects: extern "C" fn(*mut Il2CppObject, bool) -> Array<*mut Il2CppObject> = unsafe { std::mem::transmute(find_objects_addr) };
                let objects = find_objects(parts_type, false);
                if !objects.this.is_null() && objects.len() > 0 {
                    let get_is_playing_addr = symbols::get_method_addr_cached(parts_class, c"get_IsPlaying", 0);
                    if get_is_playing_addr != 0 {
                        let get_is_playing: extern "C" fn(*mut Il2CppObject) -> bool = unsafe { std::mem::transmute(get_is_playing_addr) };
                        let slice = unsafe { objects.as_slice() };
                        for obj in slice {
                            if !obj.is_null() && get_is_playing(*obj) {
                                return true;
                            }
                        }
                    }
                }
            }
        }
    }

    let go_class = cached_class(&GO_CLASS, c"UnityEngine.CoreModule.dll", c"UnityEngine", c"GameObject");
    if !go_class.is_null() {
        let find_addr = symbols::get_method_addr_cached(go_class, c"Find", 1);
        if find_addr != 0 {
            let find: extern "C" fn(*mut Il2CppString) -> *mut Il2CppObject = unsafe { std::mem::transmute(find_addr) };
            let balloon_path = "/Gallop.GameSystem/SystemManagerRoot/SystemSingleton/UIManager/GameCanvas/MainCanvas/EpisodeCharacterView(Clone)/ContentsRoot/PartsEpisodeList/MidArea/BalloonRoot".to_il2cpp_string();
            if !find(balloon_path).is_null() {
                return true;
            }
        }
    }

    false
}

// private AudioPlayback PlayInternal(SoundGroup group, RequestCueInfo cueInfo, PlayParameters playParam, AutoStopType stopType) { }
type PlayInternalFn = extern "C" fn(this: *mut Il2CppObject, group: SoundGroup,
    cue_info: *mut RequestCueInfo, play_param: *mut Il2CppObject, stop_type: i32
) -> AudioPlayback;
extern "C" fn PlayInternal(this: *mut Il2CppObject, group: SoundGroup,
    cue_info: *mut RequestCueInfo, play_param: *mut Il2CppObject, stop_type: i32
) -> AudioPlayback {
    let result = get_orig_fn!(PlayInternal, PlayInternalFn)(this, group, cue_info, play_param, stop_type);

    if group == SoundGroup::Voice && !cue_info.is_null() && Hachimi::instance().config.load().caption.caption_enable {
        let do_log = captions::Captions::show_log_enabled();

        // ── Live scene suppression ──────────────────────────────────────────
        if crate::core::gui::IS_LIVE_SCENE.load(std::sync::atomic::Ordering::Relaxed) {
            if do_log {
                info!("[captions] SKIP  | reason=IS_LIVE_SCENE active");
            }
        } else {
            let cue_sheet_ptr = unsafe { *cue_info }.CueSheetName;
            if !cue_sheet_ptr.is_null() {
                let cue_sheet = unsafe { &*cue_sheet_ptr }.as_utf16str().to_string();

                let cue_name_ptr = unsafe { *cue_info }.CueName;
                let cue_name = if !cue_name_ptr.is_null() {
                    unsafe { &*cue_name_ptr }.as_utf16str().to_string()
                } else {
                    String::new()
                };

                let cue_id = unsafe { *cue_info }.CueId;

                // Suppress live vocal cues by name
                if cue_name.contains("_live_") || cue_name.starts_with("snd_voi_live") {
                    if do_log {
                        info!("[captions] SKIP  | cue_name={} reason=live vocal cue", cue_name);
                    }
                } else if let Some(last) = cue_sheet.rsplit('_').next() {
                    if last.len() >= 6 {
                        if let Ok(chara_id) = last[..4].parse::<i32>() {
                            // lookup_cst_entry applies all suppression rules (Groups 1–5)
                            if let Some(cst_entry) = lookup_cst_entry(chara_id, cue_id, &cue_sheet) {
                                let hachimi = Hachimi::instance();
                                let mut localized_text = hachimi.localized_data.load()
                                    .character_system_text_dict
                                    .get(&chara_id)
                                    .and_then(|dict| dict.get(&cst_entry.voice_id))
                                    .cloned()
                                    .unwrap_or_else(|| cst_entry.text.clone());

                                localized_text = hachimi.template_parser.eval(&localized_text);

                                let am = instance();
                                let length = if !am.is_null() {
                                    GetCueLength(am, cue_sheet_ptr, cue_id)
                                } else { 0.0 };
                                let length = if length <= 0.0 { 3.0 } else { length };

                                if is_caption_redundant() {
                                    if do_log {
                                        info!("[captions] SKIP  (redundant — native bubble/balloon active) | chara_id={} voice_id={}",
                                            chara_id, cst_entry.voice_id);
                                    }
                                } else {
                                    captions::Captions::init();
                                    captions::Captions::set_display_time(length);

                                    let config = hachimi.config.load();
                                    if do_log {
                                        info!("[captions] caption config: enable={} font_size={} color={} outline_size={} outline_color={} pos_x={} pos_y={} bg_alpha={} fallback={} lines={}",
                                            config.caption.caption_enable,
                                            config.caption.caption_font_size,
                                            config.caption.caption_color,
                                            config.caption.caption_outline_size,
                                            config.caption.caption_outline_color,
                                            config.caption.caption_pos_x,
                                            config.caption.caption_pos_y,
                                            config.caption.caption_bg_alpha,
                                            config.caption.caption_fallback_enable,
                                            config.caption.caption_lines_char_count);
                                    }
                                    if config.caption.caption_fallback_enable {
                                        captions::Captions::show_wrapped(&localized_text, config.caption.caption_lines_char_count);
                                    } else {
                                        captions::Captions::show(&localized_text);
                                    }
                                    captions::Captions::reposition_scheduled();
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    result
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, AudioManager);

    let play_internal_addr = get_method_addr(AudioManager, c"PlayInternal", 4);
    new_hook!(play_internal_addr, PlayInternal);

    unsafe {
        CLASS = AudioManager;
        GET_CRIAUDIOMANAGER_ADDR = get_method_addr(AudioManager, c"get_CriAudioManager", 0);
        GET_CUE_LENGTH_ADDR = get_method_addr(AudioManager, c"GetCueLength", 2);
        _SONGPLAYBACK_FIELD = get_field_from_name(AudioManager, c"_songPlayback");
        _SONGCHARAPLAYBACKS_FIELD = get_field_from_name(AudioManager, c"_songCharaPlaybacks");
    }
}
