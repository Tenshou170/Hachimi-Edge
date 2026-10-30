use crate::{
    core::{capabilities::{self, Capability}, Hachimi},
    il2cpp::{symbols::{get_class_opt, get_method_addr_opt}, types::*}
};

use super::{ApplicationSettingSaveLoader, SaveDataManager};

type GetRaceDynamicCameraSettingDataFn = extern "C" fn(boot_mode: *mut Il2CppObject) -> bool;
extern "C" fn GetRaceDynamicCameraSettingData(boot_mode: *mut Il2CppObject) -> bool {
    if Hachimi::instance().config.load().force_allow_dynamic_camera {
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