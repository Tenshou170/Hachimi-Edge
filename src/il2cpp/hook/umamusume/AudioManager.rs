use crate::{
    core::{Hachimi, captions, live_utils::AudioPlayback},
    il2cpp::{
        ext::{Il2CppObjectExt, Il2CppStringExt, StringExt},
        symbols::{self, get_field_from_name, get_method_addr, SingletonLike},
        types::*,
    },
};
use super::{SceneDefine::ViewId, SceneManager};

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

// MasterCharacterSystemText class + method, cached at init() time.
static mut CST_CLASS: *mut Il2CppClass = 0 as _;
static mut GET_BY_CHARA_ID_ADDR: usize = 0;

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

#[repr(i32)]
pub enum Category {
    BGM = 0,
    SE = 1,
    VOICE = 2,
    JIKKYO = 3,
    LIVE = 4
}

def_field_value_accessors!(get__bgmPlayback, set__bgmPlayback, _BGMPLAYBACK_FIELD, crate::il2cpp::hook::Cute_Cri_Assembly::AudioPlayback::AudioPlayback_t);
def_field_object_accessors!(get__atomSourceArrayBGM, set__atomSourceArrayBGM, _ATOMSOURCEARRAYBGM_FIELD, Il2CppArray);

def_method_wrapper_fn!(GetVolume, GET_VOLUME_ADDR, f32, category: Category);
def_method_wrapper_fn!(
    PlayBgmFromName, PLAY_BGM_FROM_NAME_ADDR, (),
    this: *mut Il2CppObject, cue_name: *mut *mut Il2CppString, is_loop: bool, volume: f32,
    fade_in_time: f32, fade_out_time: f32, start_time: f32, is_cross_fade: bool, stop_type: i32
);

pub fn race_slider_music_base() -> bool {
    let audio_manager = instance();
    if audio_manager.is_null() { return false; }

    let mut bgm = get__bgmPlayback(audio_manager);
    if bgm.criAtomExPlayback.id == 0 { return false; }

    let mut num_samples: i64 = 0;
    let mut sampling_rate: i32 = 0;
    if !crate::il2cpp::hook::Cute_Cri_Assembly::AudioPlayback::GetNumPlayedSamples(&mut bgm, &mut num_samples, &mut sampling_rate) {
        return false;
    }
    if sampling_rate <= 0 { return false; }

    let secs = num_samples as f32 / sampling_rate as f32;
    if !secs.is_finite() || secs < 0.0 { return false; }

    crate::core::gui::RACE_SLIDER_MUSIC_TIME.store(secs.to_bits(), std::sync::atomic::Ordering::Release);
    true
}

pub fn play_race_bgm_cue(cue_name: *mut Il2CppString, position_secs: f32, bgm_volume: f32) -> bool {
    let audio_manager = instance();
    if audio_manager.is_null() { return false; }
    if cue_name.is_null() { return false; }

    let position = if position_secs.is_finite() && position_secs > 0.0 { position_secs } else { 0.0 };
    let volume = if bgm_volume.is_finite() && bgm_volume >= 0.0 { bgm_volume } else { 1.0 };
    let mut cue_name = cue_name;
    PlayBgmFromName(
        audio_manager, &mut cue_name, true, volume, 0.1, 0.1, position, false, 0
    );
    true
}

pub fn resync_race_music(race_manager: *mut Il2CppObject, target_time: f32) -> bool {
    use crate::il2cpp::hook::Cute_Cri_Assembly::AtomSourceEx;
    use crate::il2cpp::hook::umamusume::{RaceBGMController, RaceManager, RaceSoundReplay};
    use crate::core::utils::{race_seek_seh, race_seek_stage};
    use std::sync::atomic::Ordering;

    let race_sound = RaceManager::get_RaceSound(race_manager);
    if !RaceSoundReplay::is_replay_sound(race_sound) { return true; }

    let bgm_controller = RaceSoundReplay::get_BGMController(race_sound);
    if !RaceBGMController::is_bgm_controller(bgm_controller) { return true; }

    race_seek_seh(|| {
        race_seek_stage(12); // music_volume
        let bgm_volume = RaceSoundReplay::GetBGMVolume(race_sound);
        let second_start = RaceBGMController::get_secondBgmStartTime(bgm_controller);
        let race_base = f32::from_bits(crate::core::gui::RACE_SLIDER_DRAG_START_TIME.load(Ordering::Acquire));
        let music_valid = crate::core::gui::RACE_SLIDER_MUSIC_VALID.load(Ordering::Acquire);
        let music_base = f32::from_bits(crate::core::gui::RACE_SLIDER_MUSIC_TIME.load(Ordering::Acquire));

        let audio_manager = instance();
        if audio_manager.is_null() { return; }

        let sources_ptr = get__atomSourceArrayBGM(audio_manager);
        if sources_ptr.is_null() { return; }
        let sources: crate::il2cpp::symbols::Array<*mut Il2CppObject> = crate::il2cpp::symbols::Array::from(sources_ptr);

        race_seek_stage(13); // music_sweep
        for source in unsafe { sources.as_slice() }.iter() {
            if source.is_null() { continue; }
            if !AtomSourceEx::get_IsInUse(*source) { continue; }
            AtomSourceEx::Stop(*source, 0.0, 0);
        }

        race_seek_stage(14); // music_play_cue
        if second_start.is_finite() && second_start > 0.0 && target_time >= second_start {
            let position = if race_base >= second_start && music_valid {
                music_base + (target_time - race_base)
            } else {
                target_time - second_start
            };
            play_race_bgm_cue(RaceBGMController::get_secondBgmCueName(bgm_controller), position, bgm_volume);
            RaceBGMController::set_isRequestFirstBGM(bgm_controller, true);
            RaceBGMController::set_isStoppedFirstBGM(bgm_controller, true);
            RaceBGMController::set_isPlayedSecondBGM(bgm_controller, true);
        } else {
            let delay = RaceBGMController::get_firstBGMDelayTime(bgm_controller);
            let first_stop = RaceBGMController::get_firstBgmStopTime(bgm_controller);
    
            if target_time >= delay {
                let position = if race_base < first_stop && music_valid {
                    music_base + (target_time - race_base)
                } else {
                    target_time - delay
                };
                play_race_bgm_cue(RaceBGMController::get_firstBgmCueName(bgm_controller), position, bgm_volume);
                RaceBGMController::set_isRequestFirstBGM(bgm_controller, true);
                RaceBGMController::set_isStoppedFirstBGM(bgm_controller, false);
            } else {
                RaceBGMController::set_isRequestFirstBGM(bgm_controller, false);
                RaceBGMController::set_isStoppedFirstBGM(bgm_controller, false);
            }
            RaceBGMController::set_isPlayedSecondBGM(bgm_controller, false);
        }

        if (crate::core::Hachimi::instance().game.region == crate::core::game::Region::Japan || crate::core::Hachimi::instance().game.region == crate::core::game::Region::Taiwan)
            && RaceBGMController::is_played_first_trigger_bgm_field_valid() {
            let trigger_start = RaceBGMController::get_firstTriggerBgmPlayStartTime(bgm_controller);
            RaceBGMController::set_isPlayedFirstTriggerBgm(bgm_controller, target_time >= trigger_start);
        }

        race_seek_stage(0); // idle
    })
}

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
    "_home_",
    "_tc_",
    "_title_",
    "_factorresearch_",
    "_kakao_",
    "_gacha_",
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
const EXCEPT_TRAINING_SCENE_ALLOWED_VIEW_IDS: &[i32] = &[ViewId::ChampionsLobby as i32];

// Group 5 — View ID filter exceptions
const EXCEPT_VIEW_IDS: &[i32] = &[
    ViewId::CharacterNoteMain as i32, // 5212: Archive — Voices
    ViewId::RouletteDerby as i32,     // 8103: Prize Derby / Event Roulette
    ViewId::CharacterCardLimitBreakCut as i32, // 5213: Limit Break / Potential
];

/// Returns true if captions should be suppressed for the given ViewId.
/// Suppresses story/visual novel views where character dialogue is already
/// presented via native localized story textboxes.
fn is_view_id_suppressed(view_id: i32) -> bool {
    // Whitelist / Exception views where captions must always be permitted:
    if view_id == ViewId::CharacterNoteMain as i32
        || view_id == ViewId::RouletteDerby as i32
        || view_id == ViewId::CharacterCardLimitBreakCut as i32
    {
        return false;
    }

    match view_id {
        // Story / Visual Novel views with native dialog textboxes
        // (excluding MainStoryPaddock 3310)
        id if (ViewId::Story as i32..=3309).contains(&id) || (3311..=3999).contains(&id) => true,

        _ => false,
    }
}

/// Fetches the current SceneManager view ID. Returns 0 on failure.
fn get_current_view_id() -> i32 {
    let sm = SceneManager::instance();
    if sm.is_null() { return 0; }
    SceneManager::GetCurrentViewId(sm)
}

/// Looks up the MasterCharacterSystemText entry for the given character + cue,
/// applying all suppression/exception rules. Returns `None` if suppressed.
fn lookup_cst_entry(chara_id: i32, cue_id: i32, cue_sheet: &str, cue_name: &str) -> Option<CstEntry> {
    let do_log = captions::Captions::show_log_enabled();

    // ── Rule 1 — Cue name pattern (cheapest check, before any IL2CPP calls) ──
    let search_name = if !cue_name.is_empty() { cue_name } else { cue_sheet };
    if let Some(&pattern) = SUPPRESS_CUE_NAME_PATTERNS.iter().find(|&&p| search_name.contains(p) || cue_sheet.contains(p)) {
        if do_log {
            info!("[captions] SKIP  | cue_sheet={} cue_name={} reason=cue pattern \"{}\"",
                cue_sheet, cue_name, pattern);
        }
        return None;
    }

    // Use class + method addr cached at init() time.
    let get_by_chara_id_addr = unsafe { GET_BY_CHARA_ID_ADDR };
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
            if !cue_sheet.starts_with(&item_cue_sheet) && !cue_name.starts_with(&item_cue_sheet) { continue; }

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
                        chara_id, voice_id, cue_id, item_cue_id, search_name, voice_id);
                }
                break;
            }

            // ── Rule 3 — NPC / system character range ────────────────────────
            if chara_id >= SUPPRESS_NPC_CHARA_ID_THRESHOLD
                && !EXCEPT_NPC_ALLOW_VOICE_IDS.contains(&voice_id)
            {
                if do_log {
                    info!("[captions] SKIP  | chara_id={} voice_id={} cue_id={} item_cue_id={} cue_name={} reason=NPC chara_id >= {} (voice_id={})",
                        chara_id, voice_id, cue_id, item_cue_id, search_name,
                        SUPPRESS_NPC_CHARA_ID_THRESHOLD, voice_id);
                }
                break;
            }

            // ── Rules 4 & 5 require the current view ID ─────────────────────
            // Fetch it exactly once, only when we've passed rules 1–3.
            let current_view_id = get_current_view_id();

            // ── Rule 4 — Global view ID force-show ──────────────────────────
            let force_show_view = !EXCEPT_VIEW_IDS.is_empty()
                && EXCEPT_VIEW_IDS.contains(&current_view_id);

            // ── Rule 5 — Global view ID suppression ─────────────────────────
            let suppressed_view = !force_show_view && is_view_id_suppressed(current_view_id);
            if suppressed_view {
                if do_log {
                    info!("[captions] SKIP  | chara_id={} voice_id={} cue_id={} item_cue_id={} view_id={} cue_name={} reason=suppressed view_id {}",
                        chara_id, voice_id, cue_id, item_cue_id, current_view_id, search_name, current_view_id);
                }
                break;
            }

            // ── Rule 6 — Training scene filter ──────────────────────────────
            let mut suppressed_training = false;
            let mut suppressed_training_reason = "";
            if !force_show_view
                && (search_name.contains("_training_") || cue_sheet.contains("_training_"))
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
                        chara_id, voice_id, cue_id, item_cue_id, current_view_id, search_name,
                        if force_show_view { " [force-show view]" } else { "" }
                    );
                } else {
                    info!(
                        "[captions] SKIP  | chara_id={} voice_id={} cue_id={} item_cue_id={} view_id={} cue_name={} reason={}",
                        chara_id, voice_id, cue_id, item_cue_id, current_view_id, search_name,
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

fn has_active_speech_bubble() -> bool {
    let current_view_id = get_current_view_id();
    // Views where captions must always be permitted:
    if current_view_id == ViewId::CharacterNoteMain as i32
        || current_view_id == ViewId::RouletteDerby as i32
        || current_view_id == ViewId::CharacterCardLimitBreakCut as i32
    {
        return false;
    }

    use crate::il2cpp::hook::{
        umamusume::PartsCharaMessageBase,
        UnityEngine_CoreModule::{GameObject, Object},
    };

    // 1. Check for active PartsCharaMessageBase speech bubble components actively playing
    let parts_type = PartsCharaMessageBase::type_object();
    if !parts_type.is_null() {
        let objects = Object::FindObjectsOfType(parts_type, false);
        if !objects.this.is_null() && objects.len() > 0 {
            for obj in unsafe { objects.as_slice() } {
                if !obj.is_null() && PartsCharaMessageBase::get_IsPlaying(*obj) {
                    return true;
                }
            }
        }
    }

    // 2. Check for Episode character / story list balloon root
    let balloon_path = "/Gallop.GameSystem/SystemManagerRoot/SystemSingleton/UIManager/GameCanvas/MainCanvas/EpisodeCharacterView(Clone)/ContentsRoot/PartsEpisodeList/MidArea/BalloonRoot".to_il2cpp_string();
    let balloon = GameObject::Find(balloon_path);
    if !balloon.is_null() {
        return true;
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
                            if let Some(cst_entry) = lookup_cst_entry(chara_id, cue_id, &cue_sheet, &cue_name) {
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

                                if has_active_speech_bubble() {
                                    if do_log {
                                        info!("[captions] SKIP  (redundant — speech bubble active on page) | chara_id={} voice_id={}",
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
    get_class_or_return!(umamusume, Gallop, MasterCharacterSystemText);

    let play_internal_addr = get_method_addr(AudioManager, c"PlayInternal", 4);
    new_hook!(play_internal_addr, PlayInternal);

    unsafe {
        CLASS = AudioManager;
        GET_CRIAUDIOMANAGER_ADDR = get_method_addr(AudioManager, c"get_CriAudioManager", 0);
        GET_CUE_LENGTH_ADDR = get_method_addr(AudioManager, c"GetCueLength", 2);
        PLAY_BGM_FROM_NAME_ADDR = get_method_addr(AudioManager, c"PlayBgmFromName", 8);
        GET_VOLUME_ADDR = get_method_addr(AudioManager, c"GetVolume", 1);

        _SONGPLAYBACK_FIELD = get_field_from_name(AudioManager, c"_songPlayback");
        _SONGCHARAPLAYBACKS_FIELD = get_field_from_name(AudioManager, c"_songCharaPlaybacks");
        _BGMPLAYBACK_FIELD = get_field_from_name(AudioManager, c"_bgmPlayback");
        _ATOMSOURCEARRAYBGM_FIELD = get_field_from_name(AudioManager, c"_atomSourceArrayBGM");

        // Cache MasterCharacterSystemText lookups so lookup_cst_entry pays
        // no class-resolution cost on the hot caption path.
        CST_CLASS = MasterCharacterSystemText;
        GET_BY_CHARA_ID_ADDR = get_method_addr(MasterCharacterSystemText, c"GetByCharaId", 1);
    }
}
