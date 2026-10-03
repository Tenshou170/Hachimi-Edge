use std::sync::atomic::{AtomicBool, Ordering};

use crate::il2cpp::{
    symbols::{get_field_from_name, get_method_addr, SingletonLike},
    types::*
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

def_field_object_accessors!(get get__horseManager, _HORSEMANAGER_FIELD, Il2CppObject);

def_method_wrapper_fn!(get_RaceInfo, GET_RACE_INFO_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);
def_method_wrapper_fn!(get_RaceSound, GET_RACE_SOUND_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);
def_method_wrapper_fn!(get_RaceView, GET_RACE_VIEW_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);
def_method_wrapper_fn!(get_RaceMainView, GET_RACEMAINVIEW_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);
def_method_wrapper_fn!(get_IsSkipToRaceEndCutIn, GET_IS_SKIP_TO_RACE_END_CUT_IN_ADDR, bool, this: *mut Il2CppObject);
def_method_wrapper_fn!(set_IsSkipToRaceEndCutIn, SET_IS_SKIP_TO_RACE_END_CUT_IN_ADDR, (), this: *mut Il2CppObject, value: bool);
def_method_wrapper_fn!(get_IsAfterRaceEndCutIn, GET_IS_AFTER_RACE_END_CUT_IN_ADDR, bool, this: *mut Il2CppObject);

static WAS_FINISHED: AtomicBool = AtomicBool::new(false);

pub fn is_race_finished(race_manager: *mut Il2CppObject) -> bool {
    if race_manager.is_null() { return true; }
    let skip_cutin = get_IsSkipToRaceEndCutIn(race_manager);
    let after_cutin = get_IsAfterRaceEndCutIn(race_manager);
    let finished = skip_cutin || after_cutin;
    if finished {
        if !WAS_FINISHED.swap(true, Ordering::Relaxed) {
            info!("RaceManager::is_race_finished: true (skip_cutin={skip_cutin}, after_cutin={after_cutin})");
        }
    } else {
        WAS_FINISHED.store(false, Ordering::Relaxed);
    }
    finished
}

/// Clear leftover skip-to-race-end state at race init. Normally the game only ever
/// sets this flag at the very end of a race, right before its RaceManager is torn
/// down - but a seek that crossed the finish line can latch it mid-replay, and if
/// the instance (or an uncleared copy of the flag) survives into the next race, that
/// race fast-forwards to the end instantly. No-op when the flag doesn't exist on this
/// build (setter resolution failed) or there is no manager yet.
pub fn clear_race_end_skip_state(race_manager: *mut Il2CppObject) {
    if race_manager.is_null() {
        return;
    }
    if unsafe { SET_IS_SKIP_TO_RACE_END_CUT_IN_ADDR } == 0 {
        return;
    }
    set_IsSkipToRaceEndCutIn(race_manager, false);
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, RaceManager);

    unsafe {
        CLASS = RaceManager;
        _HORSEMANAGER_FIELD = get_field_from_name(RaceManager, c"_horseManager");
        GET_RACE_INFO_ADDR = get_method_addr(RaceManager, c"get_RaceInfo", 0);
        GET_RACE_SOUND_ADDR = get_method_addr(RaceManager, c"get_RaceSound", 0);
        GET_RACE_VIEW_ADDR = get_method_addr(RaceManager, c"get_RaceView", 0);
        GET_RACEMAINVIEW_ADDR = get_method_addr(RaceManager, c"get_RaceMainView", 0);
        GET_IS_SKIP_TO_RACE_END_CUT_IN_ADDR = get_method_addr(RaceManager, c"get_IsSkipToRaceEndCutIn", 0);
        SET_IS_SKIP_TO_RACE_END_CUT_IN_ADDR = get_method_addr(RaceManager, c"set_IsSkipToRaceEndCutIn", 1);
        GET_IS_AFTER_RACE_END_CUT_IN_ADDR = get_method_addr(RaceManager, c"get_IsAfterRaceEndCutIn", 0);
    }
}
