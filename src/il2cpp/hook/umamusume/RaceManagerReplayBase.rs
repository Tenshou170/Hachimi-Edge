use std::sync::atomic::Ordering;
use crate::{
    core::{Hachimi, game::Region, gui, utils::{clear_il2cpp_list, race_seek_seh, race_seek_stage}},
    il2cpp::{
        symbols::{get_field_from_name, get_method_addr, Array, IList},
        types::*
    }
};
use super::{
    HorseRaceInfo,
    RaceEventPlayer,
    RaceHorseManagerBase,
    RaceHorseManagerReplay,
    RaceManager,
    RaceSimulateEventData,
    RaceSimulateReader,
    Jikkyo,
    JikkyoControllerBase, 
    RaceMainViewController,
    RaceSimulateData,
    RaceUI,
    RaceUIMiniMap,
    RaceViewReplay,
    SimulateEventType::SimulateEventType,
    SkillManager,
};

def_method_wrapper_fn!(ForceSetRaceTime, FORCE_SET_RACE_TIME_ADDR, (), this: *mut Il2CppObject, time: f32, isCheckEvent: bool);
def_method_wrapper_fn!(IsPaused, IS_PAUSED_ADDR, bool, this: *mut Il2CppObject);
def_method_wrapper_fn!(PauseRace, PAUSE_RACE_ADDR, (), this: *mut Il2CppObject);
def_method_wrapper_fn!(ResumeRace, RESUME_RACE_ADDR, (), this: *mut Il2CppObject);
def_method_wrapper_fn!(get_IsPlayingCutIn, IS_PLAYING_CUT_IN_ADDR, bool, this: *mut Il2CppObject);
def_method_wrapper_fn!(get_Jikkyo, GET_JIKKYO_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);
def_method_wrapper_fn!(UpdateCameraEventAll, UPDATE_CAMERA_EVENT_ALL_ADDR, (), this: *mut Il2CppObject, force_update: bool);
def_method_wrapper_fn!(LateUpdateCameraEventAll, LATE_UPDATE_CAMERA_EVENT_ALL_ADDR, (), this: *mut Il2CppObject, force_update: bool);
def_method_wrapper_fn!(IsCutInPlayingOrReserved, IS_CUT_IN_PLAYING_OR_RESERVED_ADDR, bool, this: *mut Il2CppObject);
def_method_wrapper_fn!(UpdateHorses, UPDATE_HORSES_ADDR, (), this: *mut Il2CppObject, elapsed_time: f32);
def_method_wrapper_fn!(UpdateHorseDatas, UPDATE_HORSE_DATAS_ADDR, (), this: *mut Il2CppObject, elapsed_time: f32);
def_method_wrapper_fn!(UpdateHorseModels, UPDATE_HORSE_MODELS_ADDR, (), this: *mut Il2CppObject);

def_field_object_accessors!(get__eventPlayer, set__eventPlayer, EVENT_PLAYER_FIELD, Il2CppObject);

/// Longest plausible race (generous). Totals outside `(0, MAX_RACE_SECS]` are
/// treated as garbage rather than trusted as a seek bound - a transiently bogus
/// `GetLastFrameTime()` previously became the slider's range and let seeks land
/// far past the finish line.
pub const MAX_RACE_SECS: f32 = 900.0;

/// Hard ceiling for seek targets, computed on the game thread: the moment the last
/// horse covers the course (the replay's terminal goal - seeking at/past it fires the
/// game's finish checks mid-seek and wedges the replay into its race-end state), minus
/// a small margin, itself bounded by the sim's last frame. Falls back to
/// `total - 0.5` when per-horse finish times aren't readable yet; returns 0.0 when no
/// sane total exists, meaning "cannot bound" (callers must then refuse to seek).
///
/// Game thread only (IL2CPP calls); the render thread reads the cached copy in
/// `crate::core::gui` instead.
pub fn seek_ceiling(race_manager: *mut Il2CppObject) -> f32 {
    let horse_manager = RaceManager::get__horseManager(race_manager);
    if horse_manager.is_null() || !RaceHorseManagerReplay::is_replay_manager(horse_manager) {
        return 0.0;
    }
    let reader = RaceHorseManagerReplay::get__reader(horse_manager);
    if reader.is_null() {
        return 0.0;
    }

    let total = RaceSimulateReader::GetLastFrameTime(reader);
    if !total.is_finite() || total <= 0.0 || total > MAX_RACE_SECS {
        return 0.0;
    }
    let mut ceiling = total - 0.5;

    // Tighten to the last horse's finish time when the sim can tell us. The finish
    // times must live in the same clock as the reader (`_curTime`/`GetLastFrameTime`)
    // and land just before the final frame; anything else is discarded and we keep
    // the plain `total - 0.5` bound (never worse than the old behavior).
    let course = crate::core::race_director::course_distance() as f32;
    if course > 0.0 {
        let horse_infos = RaceHorseManagerBase::GetHorseRaceInfos(horse_manager);
        if !horse_infos.is_null() {
            let horse_arr: Array<*mut Il2CppObject> = Array::from(horse_infos);
            let n = horse_arr.len();
            let mut last_finish = 0.0f32;
            for i in 0..n {
                let t = RaceSimulateReader::GetTimeByDistance(reader, i as i32, course);
                if t.is_finite() && t > last_finish {
                    last_finish = t;
                }
            }
            if last_finish > 0.0 && last_finish < total && last_finish + 15.0 >= total {
                ceiling = ceiling.min(last_finish - 0.5);
            }
        }
    }

    if ceiling < 0.0 { 0.0 } else { ceiling }
}

pub fn seek_sync(target_time: f32) -> bool {
    let race_manager = RaceManager::instance();
    if race_manager.is_null() { return true; }

    // Defense in depth: whatever the caller computed, never seek to or past the
    // terminal goal, and never seek on a nonsense target.
    let ceiling = seek_ceiling(race_manager);
    if ceiling <= 0.0 || !target_time.is_finite() || target_time < 0.0 {
        return true;
    }
    let target_time = target_time.min(ceiling);

    let event_player = get__eventPlayer(race_manager);

    let prev_time = RaceSimulateReader::replay_cur_time(race_manager)
        .unwrap_or(f32::from_bits(gui::RACE_SLIDER_LAST_APPLIED.load(Ordering::Acquire)));
    let backward = target_time < prev_time - 0.5;

    let ok = race_seek_seh(|| {
        if RaceEventPlayer::is_event_player(event_player) {
            race_seek_stage(1); // event_cursor
            RaceEventPlayer::ChangeLastEventIndexByTime(event_player, target_time);
        }

        let horse_manager = RaceManager::get__horseManager(race_manager);
        if !horse_manager.is_null() && RaceHorseManagerReplay::is_replay_manager(horse_manager) {
            let horse_infos = RaceHorseManagerBase::GetHorseRaceInfos(horse_manager);
            if !horse_infos.is_null() {
                let horse_arr: Array<*mut Il2CppObject> = Array::from(horse_infos);
                let horse_infos_slice = unsafe { horse_arr.as_slice() };
                for horse_info in horse_infos_slice.iter() {
                    if !horse_info.is_null() && HorseRaceInfo::get__phase(*horse_info) == 4 {
                        HorseRaceInfo::set__phase(*horse_info, 3);
                    }
                }
            }
        }
        // NOTE: this intentionally does NOT clear race_director's race-finished
        // latch any more. Clearing it here let a seek resurrect the HUD after the
        // race was already over (the player horse had been rewound and could never
        // finish again, so the latch never re-fired). Seeks can no longer cross the
        // finish (ceiling above), so the latch can only be genuinely set now.

        race_seek_stage(2); // race_time
        ForceSetRaceTime(race_manager, target_time, true);
        race_seek_stage(3); // horses
        UpdateHorses(race_manager, target_time);
        race_seek_stage(4); // horse_datas
        UpdateHorseDatas(race_manager, target_time);
        race_seek_stage(5); // models
        UpdateHorseModels(race_manager);
        race_seek_stage(6); // camera_events
        UpdateCameraEventAll(race_manager, true);
        race_seek_stage(7); // late_camera_events
        LateUpdateCameraEventAll(race_manager, true);
        race_seek_stage(8); // used_skills
        resync_used_skills(race_manager, target_time);

        race_seek_stage(9); // jikkyo_sync
        let jikkyo = get_Jikkyo(race_manager);
        if jikkyo.is_null() { return; }

        if backward {
            Jikkyo::SkipToStart(jikkyo);
        } else {
            JikkyoControllerBase::ClearDisplay(jikkyo);
            JikkyoControllerBase::ClearVoice(jikkyo);
            JikkyoControllerBase::ClearReserve(jikkyo);
        }

        race_seek_stage(10); // view_rearm
        let view = RaceManager::get_RaceView(race_manager);
        if view.is_null() { return; }
        RaceViewReplay::set_lastSpurtProcessed(view, false);

        if backward {
            race_seek_stage(11); // distance_repair
            // The game only rewinds `_distance` for unfinished horses; finished ones
            // keep stale values (observed at ~53 km) that explode the timing tower's
            // gap math. Repair any horse whose distance is inconsistent with where
            // the sim says it should be at the target time.
            repair_horse_distances(race_manager, target_time);

            race_seek_stage(12); // minimap_rearm
            let race_main_view = RaceManager::get_RaceMainView(race_manager);
            if race_main_view.is_null() { return; }

            let race_ui = RaceMainViewController::get__raceUI(race_main_view);
            if race_ui.is_null() { return; }

            let minimap = RaceUI::get__minimap(race_ui);
            if minimap.is_null() { return; }

            RaceUIMiniMap::set_hasMiniMapShown(minimap, false);
            RaceUIMiniMap::set_hasMiniMapHidden(minimap, false);
        }

        race_seek_stage(0); // idle
    });

    ok
}

/// Post-seek repair for backward seeks: force any horse whose `_distance` is
/// non-finite, negative, absurdly past the course, or still far ahead of where the
/// sim's time->distance table says it should be at `target_time`, back to the sim's
/// own value. Called inside `race_seek_seh`, so a fault here is reported by stage.
fn repair_horse_distances(race_manager: *mut Il2CppObject, target_time: f32) {
    let horse_manager = RaceManager::get__horseManager(race_manager);
    if horse_manager.is_null() || !RaceHorseManagerReplay::is_replay_manager(horse_manager) {
        return;
    }
    let reader = RaceHorseManagerReplay::get__reader(horse_manager);
    if reader.is_null() {
        return;
    }
    let horse_infos = RaceHorseManagerBase::GetHorseRaceInfos(horse_manager);
    if horse_infos.is_null() {
        return;
    }

    let course = crate::core::race_director::course_distance() as f32;
    let horse_arr: Array<*mut Il2CppObject> = Array::from(horse_infos);
    for (i, horse_info) in unsafe { horse_arr.as_slice() }.iter().enumerate() {
        if horse_info.is_null() {
            continue;
        }
        // horseIndex == gate - 1 == infos index (same mapping resync_used_skills uses).
        let expected = RaceSimulateReader::GetDistance(reader, i as i32, target_time);
        if !expected.is_finite()
            || expected < 0.0
            || (course > 0.0 && expected > course + 500.0)
        {
            continue;
        }
        let cur = HorseRaceInfo::get__distance(*horse_info);
        let corrupt = !cur.is_finite()
            || cur < -1.0
            || (course > 0.0 && cur > course + 500.0)
            || cur > expected + 100.0;
        if corrupt {
            HorseRaceInfo::set__distance(*horse_info, expected);
        }
    }
}

fn resync_used_skills(race_manager: *mut Il2CppObject, target_time: f32) {
    if Hachimi::instance().game.region != Region::Japan {
        return;
    }

    let horse_manager = RaceManager::get__horseManager(race_manager);
    if horse_manager.is_null() { return; }
    if !RaceHorseManagerReplay::is_replay_manager(horse_manager) { return; }

    let horse_infos = RaceHorseManagerBase::GetHorseRaceInfos(horse_manager);
    if horse_infos.is_null() { return; }
    let horse_arr: Array<*mut Il2CppObject> = Array::from(horse_infos);
    let horse_count = horse_arr.len();
    if horse_count == 0 { return; }

    let horse_infos_slice = unsafe { horse_arr.as_slice() };
    for horse_info in horse_infos_slice.iter() {
        let skill_manager = HorseRaceInfo::get__skillManager(*horse_info);
        if skill_manager.is_null() { continue; }
        let used_list = SkillManager::GetUsedSkillIdList(skill_manager);
        if used_list.is_null() { continue; }
        clear_il2cpp_list(used_list);
    }

    let reader = RaceHorseManagerReplay::get__reader(horse_manager);
    if reader.is_null() { return; }

    let sim_data = RaceSimulateReader::get__simData(reader);
    if sim_data.is_null() { return; }

    let event_list = RaceSimulateData::get__simEvDataList(sim_data);
    let Some(event_list) = IList::<*mut Il2CppObject>::new(event_list) else {
        return;
    };

    for event in event_list.iter() {
        if event.is_null() { continue; }
        if RaceSimulateEventData::get_type(event) != SimulateEventType::Skill { continue; }
        if RaceSimulateEventData::get_frameTime(event) > target_time { continue; }

        let mut horse_idx: i32 = -1;
        let mut skill_id: i32 = 0;
        let mut detail_index: i32 = 0;
        let mut time_int: i32 = 0;
        let mut target_flags: i32 = 0;
        let mut caller_skill_id: i32 = 0;
        let mut activate_type: i32 = 0;
        let mut ability_value_status: i32 = 0;
        let mut ability_time_status: i32 = 0;
        RaceEventPlayer::GetSkillEventParam(
            event,
            &mut horse_idx,
            &mut skill_id,
            &mut detail_index,
            &mut time_int,
            &mut target_flags,
            &mut caller_skill_id,
            &mut activate_type,
            &mut ability_value_status,
            &mut ability_time_status
        );

        if horse_idx < 0 || horse_idx as usize >= horse_count { continue; }
        let horse_info = horse_infos_slice[horse_idx as usize];
        if horse_info.is_null() { continue; }

        let skill_manager = HorseRaceInfo::get__skillManager(horse_info);
        if skill_manager.is_null() { continue; }
        SkillManager::AddUsedSkillId(skill_manager, skill_id);
    }
}

pub fn playback_gated(race_manager: *mut Il2CppObject) -> bool {
    HorseRaceInfo::is_start_dash_instance(race_manager) ||
    get_IsPlayingCutIn(race_manager)
}

pub fn toggle_playback() {
    if !RaceHorseManagerBase::is_race_active() {
        return;
    }

    let race_manager = RaceManager::instance();
    if race_manager.is_null() { return; }

    if playback_gated(race_manager) { return; }

    if IsPaused(race_manager) {
        ResumeRace(race_manager);
    } else {
        PauseRace(race_manager);
    }
}

type RaceManagerReplayBase_OnClickSkipButtonFn = extern "C" fn(this: *mut Il2CppObject);
extern "C" fn RaceManagerReplayBase_OnClickSkipButton(this: *mut Il2CppObject) {
    // Also fires when skipping the pre-race intro, not just skip-to-end - only treat it
    // as "race finished" if the gate has already opened.
    if crate::core::race_director::is_gate_open() {
        crate::core::race_director::set_race_finished(true);
        crate::core::gui::reset_race_slider();
    }
    get_orig_fn!(RaceManagerReplayBase_OnClickSkipButton, RaceManagerReplayBase_OnClickSkipButtonFn)(this);
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, RaceManagerReplayBase);

    unsafe {
        FORCE_SET_RACE_TIME_ADDR = get_method_addr(RaceManagerReplayBase, c"ForceSetRaceTime", 2);
        IS_PAUSED_ADDR = get_method_addr(RaceManagerReplayBase, c"IsPaused", 0);
        PAUSE_RACE_ADDR = get_method_addr(RaceManagerReplayBase, c"PauseRace", 0);
        RESUME_RACE_ADDR = get_method_addr(RaceManagerReplayBase, c"ResumeRace", 0);
        IS_PLAYING_CUT_IN_ADDR = get_method_addr(RaceManagerReplayBase, c"get_IsPlayingCutIn", 0);
        GET_JIKKYO_ADDR = get_method_addr(RaceManagerReplayBase, c"get_Jikkyo", 0);
        UPDATE_CAMERA_EVENT_ALL_ADDR = get_method_addr(RaceManagerReplayBase, c"UpdateCameraEventAll", 1);
        LATE_UPDATE_CAMERA_EVENT_ALL_ADDR = get_method_addr(RaceManagerReplayBase, c"LateUpdateCameraEventAll", 1);
        IS_CUT_IN_PLAYING_OR_RESERVED_ADDR = get_method_addr(RaceManagerReplayBase, c"IsCutInPlayingOrReserved", 0);
        UPDATE_HORSES_ADDR = get_method_addr(RaceManagerReplayBase, c"UpdateHorses", 1);
        UPDATE_HORSE_DATAS_ADDR = get_method_addr(RaceManagerReplayBase, c"UpdateHorseDatas", 1);
        UPDATE_HORSE_MODELS_ADDR = get_method_addr(RaceManagerReplayBase, c"UpdateHorseModels", 0);
        EVENT_PLAYER_FIELD = get_field_from_name(RaceManagerReplayBase, c"_eventPlayer");
    }

    let OnClickSkipButton_addr = get_method_addr(RaceManagerReplayBase, c"OnClickSkipButton", 0);
    new_hook!(OnClickSkipButton_addr, RaceManagerReplayBase_OnClickSkipButton);
}
