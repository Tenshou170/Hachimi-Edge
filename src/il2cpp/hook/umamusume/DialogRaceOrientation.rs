use crate::{
    core::Hachimi,
    il2cpp::{symbols::{get_class_opt, get_method_addr_opt}, types::*}
};

use super::RaceInfo;

type SetupAndOpenFn = extern "C" fn(
    this: *mut Il2CppObject, dialog_data: *mut Il2CppObject, on_selected: *mut Il2CppObject,
    on_cancel: *mut Il2CppObject, is_special_unlock_race: bool, race_info: *mut Il2CppObject
);
extern "C" fn SetupAndOpen(
    this: *mut Il2CppObject, dialog_data: *mut Il2CppObject, on_selected: *mut Il2CppObject,
    on_cancel: *mut Il2CppObject, is_special_unlock_race: bool, race_info: *mut Il2CppObject
) {
    // Don't spoof LoH race type for story races (upstream #50)
    let force_allow_dynamic_camera = Hachimi::instance().config.load().force_allow_dynamic_camera
        && !race_info.is_null()
        && !RaceInfo::get_IsStoryRace(race_info);
    let mut orig_race_type = None;
    if force_allow_dynamic_camera {
        orig_race_type = Some(RaceInfo::get_RaceType(race_info));
        RaceInfo::set_RaceType(race_info, 16); // spoof LoH race
    }

    get_orig_fn!(SetupAndOpen, SetupAndOpenFn)(
        this, dialog_data, on_selected, on_cancel, is_special_unlock_race, race_info
    );

    if let Some(race_type) = orig_race_type {
        if !race_info.is_null() {
            RaceInfo::set_RaceType(race_info, race_type);
        }
    }
}

pub fn init(umamusume: *const Il2CppImage) {
    let Some(DialogRaceOrientation) = get_class_opt(umamusume, c"Gallop", c"DialogRaceOrientation") else {
        return;
    };

    // The JP overload passes the RaceInfo the dynamic-camera spoof needs. Other
    // clients call a 4-argument overload without it, so there is nothing to
    // spoof there and the hook is skipped.
    if let Some(addr) = get_method_addr_opt(DialogRaceOrientation, c"SetupAndOpen", 5) {
        new_hook!(addr, SetupAndOpen);
    }
}