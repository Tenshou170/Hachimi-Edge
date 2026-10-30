use crate::{
    core::capabilities::{self, Capability},
    il2cpp::{symbols::{get_class_opt, get_method_addr_opt}, types::*}
};

static mut GET_ISTRYRACEDYNAMICCAMERA_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_IsTryRaceDynamicCamera, GET_ISTRYRACEDYNAMICCAMERA_ADDR, bool, this: *mut Il2CppObject);

pub fn init(umamusume: *const Il2CppImage) {
    let Some(ApplicationSettingSaveLoader) = get_class_opt(umamusume, c"Gallop", c"ApplicationSettingSaveLoader") else {
        return;
    };

    unsafe {
        GET_ISTRYRACEDYNAMICCAMERA_ADDR = get_method_addr_opt(ApplicationSettingSaveLoader, c"get_IsTryRaceDynamicCamera", 0)
            .unwrap_or(0);
    }

    if unsafe { GET_ISTRYRACEDYNAMICCAMERA_ADDR } != 0 {
        capabilities::set(Capability::RaceDynamicCameraSetting);
    }
}