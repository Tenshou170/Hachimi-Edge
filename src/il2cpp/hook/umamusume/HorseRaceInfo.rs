use crate::{
    core::{game::Region, Hachimi},
    il2cpp::{
        symbols::{get_field_from_name, get_method_addr, Array},
        types::*,
    },
};

use super::{RaceInfo, RaceManager, RaceHorseManagerBase};

def_field_value_accessors!(get__position, set__position, POSITION_FIELD, Vector3_t);
def_field_value_accessors!(get__rotationOnLane, set__rotationOnLane, ROTATION_ON_LANE_FIELD, Quaternion_t);
def_field_object_accessors!(get__skillManager, set__skillManager, SKILL_MANAGER_FIELD, Il2CppObject);

def_field_value_accessors!(get get__hp, HP_FIELD, f32);
def_field_value_accessors!(get get__maxHp, MAX_HP_FIELD, f32);
def_field_value_accessors!(get get__curOrder, CUR_ORDER_FIELD, i32);
def_field_value_accessors!(get get__lastSpeed, LAST_SPEED_FIELD, f32);
def_field_value_accessors!(get__distance, set__distance, DISTANCE_FIELD, f32);
def_field_value_accessors!(get get__isHpEmptyOnRace, IS_HP_EMPTY_ON_RACE_FIELD, bool);
def_field_value_accessors!(get__phase, set__phase, PHASE_FIELD, i32);
def_field_value_accessors!(get get__isBadStart, IS_BAD_START_FIELD, bool);
def_field_value_accessors!(get get__isCompeteFight, IS_COMPETE_FIGHT_FIELD, bool);
def_field_value_accessors!(get get__isCompeteTop, IS_COMPETE_TOP_FIELD, bool);
def_field_value_accessors!(get get__prevOrder, PREV_ORDER_FIELD, i32);
def_field_object_accessors!(get get__horseData, HORSE_DATA_FIELD, Il2CppObject);
def_field_object_accessors!(get get__horseRaceAI, HORSE_RACE_AI_FIELD, Il2CppObject);

def_method_wrapper_fn!(get_IsStartDash, GET_ISSTARTDASH_ADDR, bool, this: *mut Il2CppObject);
def_method_wrapper_fn!(IsFinished, ISFINISHED_ADDR, bool, this: *mut Il2CppObject);

pub fn player_horse_index(horse_manager: *mut Il2CppObject, count: usize) -> usize {
    let player_idx = RaceHorseManagerBase::GetPlayerHorseIndex(horse_manager);
    if player_idx >= 0 && (player_idx as usize) < count {
        player_idx as usize
    } else {
        0
    }
}

pub fn player_horse_info(horse_manager: *mut Il2CppObject) -> Option<*mut Il2CppObject> {
    if horse_manager.is_null() {
        return None;
    }

    let horse_infos = RaceHorseManagerBase::GetHorseRaceInfos(horse_manager);
    if horse_infos.is_null() {
        return None;
    }
    let arr: Array<*mut Il2CppObject> = Array::from(horse_infos);
    if arr.len() == 0 {
        return None;
    }
    let player_idx = player_horse_index(horse_manager, arr.len());
    let player_info = unsafe { arr.as_slice()[player_idx] };
    if player_info.is_null() {
        return None;
    }

    Some(player_info)
}

pub fn is_start_dash() -> bool {
    let race_manager = RaceManager::instance();
    is_start_dash_instance(race_manager)
}

pub fn is_start_dash_instance(race_manager: *mut Il2CppObject) -> bool {
    if race_manager.is_null() { return false; }

    let horse_manager = RaceManager::get__horseManager(race_manager);
    if horse_manager.is_null() { return false; }

    match player_horse_info(horse_manager) {
        Some(player_info) => get_IsStartDash(player_info),
        None => false,
    }
}

/// Whether the player's horse has finished the race. `None` when race data is
/// currently unavailable — the game can drop `RaceManager`/`_horseManager` for a
/// single transient frame mid-race, and the one-way finished latch in
/// race_telemetry must never fire on that (it would hide the Race Director HUD
/// and playback slider for the rest of the race). Overlays hide while data is
/// missing via the null guards in the GUI `*_showing()` paths instead.
pub fn is_finished() -> Option<bool> {
    let race_manager = RaceManager::instance();
    if race_manager.is_null() { return None; }

    let horse_manager = RaceManager::get__horseManager(race_manager);
    if horse_manager.is_null() { return None; }

    let player_info = player_horse_info(horse_manager)?;
    Some(IsFinished(player_info) || reached_course_end(player_info))
}

/// The game can leave IsFinished() false after the player horse has crossed the
/// course end (observed with race-related overlays), so overlays that poll it
/// never see the finish. Treat reaching the course distance as finished too.
fn reached_course_end(player_info: *mut Il2CppObject) -> bool {
    let race_manager = RaceManager::instance();
    if race_manager.is_null() { return false; }

    let race_info = RaceManager::get_RaceInfo(race_manager);
    if race_info.is_null() { return false; }

    let course_distance = if Hachimi::instance().game.region == Region::Global {
        RaceInfo::course_distance(race_info) as f32
    } else {
        // JP/TW RaceInfo doesn't expose get_CourseDistance; sum its components.
        RaceInfo::course_only_plus_run_up(race_info) as f32
    };
    if course_distance <= 0.0 { return false; }

    get__distance(player_info) >= course_distance
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, HorseRaceInfo);

    unsafe {
        POSITION_FIELD = get_field_from_name(HorseRaceInfo, c"_position");
        ROTATION_ON_LANE_FIELD = get_field_from_name(HorseRaceInfo, c"_rotationOnLane");
        SKILL_MANAGER_FIELD = get_field_from_name(HorseRaceInfo, c"_skillManager");
        HP_FIELD = get_field_from_name(HorseRaceInfo, c"_hp");
        MAX_HP_FIELD = get_field_from_name(HorseRaceInfo, c"_maxHp");
        CUR_ORDER_FIELD = get_field_from_name(HorseRaceInfo, c"<CurOrder>k__BackingField");
        LAST_SPEED_FIELD = get_field_from_name(HorseRaceInfo, c"_lastSpeed");
        DISTANCE_FIELD = get_field_from_name(HorseRaceInfo, c"_distance");
        IS_HP_EMPTY_ON_RACE_FIELD = get_field_from_name(HorseRaceInfo, c"<IsHpEmptyOnRace>k__BackingField");
        PHASE_FIELD = get_field_from_name(HorseRaceInfo, c"_phase");
        IS_BAD_START_FIELD = get_field_from_name(HorseRaceInfo, c"<IsBadStart>k__BackingField");
        IS_COMPETE_FIGHT_FIELD = get_field_from_name(HorseRaceInfo, c"<IsCompeteFight>k__BackingField");
        IS_COMPETE_TOP_FIELD = get_field_from_name(HorseRaceInfo, c"<IsCompeteTop>k__BackingField");
        PREV_ORDER_FIELD = get_field_from_name(HorseRaceInfo, c"<PrevOrder>k__BackingField");
        HORSE_DATA_FIELD = get_field_from_name(HorseRaceInfo, c"_horseData");
        HORSE_RACE_AI_FIELD = get_field_from_name(HorseRaceInfo, c"_horseRaceAI");
        GET_ISSTARTDASH_ADDR = get_method_addr(HorseRaceInfo, c"get_IsStartDash", 0);
        ISFINISHED_ADDR = get_method_addr(HorseRaceInfo, c"IsFinished", 0);
    }
}
