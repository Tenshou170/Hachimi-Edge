/// Hooks `Gallop.Model.Component.CySpringUpdater::set_SpringUpdateMode` and
/// `get_SpringUpdateMode` to enforce the configured physics mode at the
/// per-component level. This prevents the game from resetting the mode after
/// `CySpringController::Init` has already set it.
use crate::{core::Hachimi, il2cpp::{symbols::get_method_addr, types::*}};

type SetSpringUpdateModeFn = extern "C" fn(this: *mut Il2CppObject, value: i32);
extern "C" fn set_SpringUpdateMode(this: *mut Il2CppObject, value: i32) {
    let config = Hachimi::instance().config.load();
    let effective = match config.physics_update_mode.as_ref() {
        Some(mode) => *mode as i32,
        None => value,
    };
    get_orig_fn!(set_SpringUpdateMode, SetSpringUpdateModeFn)(this, effective);
}

type GetSpringUpdateModeFn = extern "C" fn(this: *mut Il2CppObject) -> i32;
extern "C" fn get_SpringUpdateMode(this: *mut Il2CppObject) -> i32 {
    let result = get_orig_fn!(get_SpringUpdateMode, GetSpringUpdateModeFn)(this);
    let config = Hachimi::instance().config.load();
    match config.physics_update_mode.as_ref() {
        Some(mode) => *mode as i32,
        None => result,
    }
}

pub fn init(umamusume: *const Il2CppImage) {
    // Only hook if the user has configured a physics mode override
    if Hachimi::instance().config.load().physics_update_mode.is_none() {
        return;
    }

    get_class_or_return!(umamusume, "Gallop.Model.Component", CySpringUpdater);

    let set_addr = get_method_addr(CySpringUpdater, c"set_SpringUpdateMode", 1);
    let get_addr = get_method_addr(CySpringUpdater, c"get_SpringUpdateMode", 0);

    if set_addr != 0 {
        new_hook!(set_addr, set_SpringUpdateMode);
    }
    if get_addr != 0 {
        new_hook!(get_addr, get_SpringUpdateMode);
    }
}
