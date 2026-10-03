use std::sync::atomic::{AtomicBool, Ordering};

use crate::il2cpp::{
    symbols::get_method_addr,
    types::*
};

use super::RaceManager;

static RACE_ACTIVE: AtomicBool = AtomicBool::new(false);
pub fn is_race_active() -> bool {
    RACE_ACTIVE.load(Ordering::Acquire)
}

def_method_wrapper_fn!(GetHorseRaceInfos, GET_HORSE_RACE_INFOS_ADDR, *mut Il2CppArray, this: *mut Il2CppObject);
def_method_wrapper_fn!(GetPlayerHorseIndex, GET_PLAYER_HORSE_INDEX_ADDR, i32, this: *mut Il2CppObject);

type RaceHorseManagerBase_InitFn = extern "C" fn(this: *mut Il2CppObject, raceInfo: *mut Il2CppObject);
extern "C" fn RaceHorseManagerBase_Init(this: *mut Il2CppObject, raceInfo: *mut Il2CppObject) {
    // Reset all race-director telemetry atomics before marking the race active so
    // collect_frame() never sees stale GATE_OPEN/RACE_FINISHED from the prior race.
    // Called unconditionally — independent of race_director.enabled in config.
    crate::core::race_director::on_race_start();
    // Best-effort cross-race hygiene, before this race starts running:
    // - a prior seek that crossed the finish can leave the game's skip-to-race-end
    //   flag latched on a reused RaceManager, which would fast-forward this race to
    //   the end instantly (no-op when the flag doesn't exist on this build);
    // - drop the previous race's cached seek ceiling so the slider recomputes it
    //   for this race's course instead of reusing a stale bound.
    RaceManager::clear_race_end_skip_state(RaceManager::instance());
    crate::core::gui::reset_seek_ceiling();
    RACE_ACTIVE.store(true, Ordering::Release);
    get_orig_fn!(RaceHorseManagerBase_Init, RaceHorseManagerBase_InitFn)(this, raceInfo);
}

type RaceHorseManagerBase_ReleaseFn = extern "C" fn(this: *mut Il2CppObject);
extern "C" fn RaceHorseManagerBase_Release(this: *mut Il2CppObject) {
    RACE_ACTIVE.store(false, Ordering::Release);
    crate::core::race_director::set_race_finished(true);
    crate::core::race_director::set_gate_open(false);
    // Reset race slider state and input atomics immediately so the end screen is interactive
    // without waiting for the next render frame.
    crate::core::gui::reset_race_slider();
    get_orig_fn!(RaceHorseManagerBase_Release, RaceHorseManagerBase_ReleaseFn)(this);
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, RaceHorseManagerBase);

    unsafe {
        GET_HORSE_RACE_INFOS_ADDR = get_method_addr(RaceHorseManagerBase, c"GetHorseRaceInfos", 0);
        GET_PLAYER_HORSE_INDEX_ADDR = get_method_addr(RaceHorseManagerBase, c"GetPlayerHorseIndex", 0);
    }

    let Init_addr = get_method_addr(RaceHorseManagerBase, c"Init", 1);
    new_hook!(Init_addr, RaceHorseManagerBase_Init);

    let Release_addr = get_method_addr(RaceHorseManagerBase, c"Release", 0);
    new_hook!(Release_addr, RaceHorseManagerBase_Release);
}
