use crate::{
    core::{capabilities::{self, Capability}, Hachimi},
    il2cpp::{symbols::{get_class_opt, get_method_addr_opt}, types::*}
};

use super::{ApplicationSettingSaveLoader, RaceInfo, RaceManager, SaveDataManager};

fn is_story_race() -> bool {
    // RaceManager.RaceInfo is a static property, populated before RaceManager::instance()
    // exists (e.g. during RaceLoaderDynamicCamera / RaceMainViewController.InitializeView).
    let race_info = RaceManager::get_RaceInfo(0 as _);
    if race_info.is_null() {
        return false;
    }
    RaceInfo::get_IsStoryRace(race_info) || matches!(RaceInfo::get_RaceType(race_info), 2 | 3 | 4)
}

type GetRaceDynamicCameraSettingDataFn = extern "C" fn(boot_mode: i32) -> bool;
extern "C" fn GetRaceDynamicCameraSettingData(boot_mode: i32) -> bool {
    // Don't force dynamic camera for Highlight (2), OnlyGoalCapture (3), Commercial (5), or story races
    if Hachimi::instance().config.load().force_allow_dynamic_camera
        && !matches!(boot_mode, 2 | 3 | 5)
        && !is_story_race()
    {
        let save_data_manager = SaveDataManager::instance();
        if save_data_manager.is_null() { return false; }

        let save_loader = SaveDataManager::get_SaveLoader(save_data_manager);
        if save_loader.is_null() { return false; }

        ApplicationSettingSaveLoader::get_IsTryRaceDynamicCamera(save_loader)
    }
    else {
        get_orig_fn!(GetRaceDynamicCameraSettingData, GetRaceDynamicCameraSettingDataFn)(boot_mode)
    }
}

pub fn init(umamusume: *const Il2CppImage) {
    let Some(RaceUtil) = get_class_opt(umamusume, c"Gallop", c"RaceUtil") else {
        return;
    };

    let GetRaceDynamicCameraSettingData_addr = get_method_addr_opt(RaceUtil, c"GetRaceDynamicCameraSettingData", 1);
    let Some(addr) = GetRaceDynamicCameraSettingData_addr else {
        return;
    };

    capabilities::set(Capability::RaceDynamicCameraUtil);
    new_hook!(addr, GetRaceDynamicCameraSettingData);
}