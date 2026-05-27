use crate::{core::Hachimi, il2cpp::{api::il2cpp_field_static_set_value, symbols::{get_field_from_name, get_method_addr}, types::*}};

// --- static isNative field, resolved once in init() ---
static mut IS_NATIVE_FIELD: *mut FieldInfo = 0 as _;

type CctorFn = extern "C" fn();
extern "C" fn cctor() {
    get_orig_fn!(cctor, CctorFn)();

    // If cyspring_disable_native is set, overwrite the static isNative field
    // to false so the game uses the Mono (managed) physics path instead of
    // the native C++ plugin — mirrors localify's cySpringDisableNative option.
    if Hachimi::instance().config.load().cyspring_disable_native {
        let mut value: bool = false;
        unsafe {
            il2cpp_field_static_set_value(IS_NATIVE_FIELD, &mut value as *mut bool as _);
        }
    }
}

type UpdateForceFn = extern "C" fn(
    cloth_working: *mut std::ffi::c_void, stiffness_force_rate: f32, drag_force_rate: f32,
    gravity_rate: f32, wind_power: Vector3_t, wind_strength: f32,
    position_diff: Vector3_t, frame_scale: f32
);

extern "C" fn UpdateForce(
    cloth_working: *mut std::ffi::c_void, stiffness_force_rate: f32, drag_force_rate: f32,
    gravity_rate: f32, wind_power: Vector3_t, wind_strength: f32,
    position_diff: Vector3_t, mut frame_scale: f32
) {
    let config = Hachimi::instance().config.load();
    if config.physics_update_mode == Some(super::CySpringController::SpringUpdateMode::Mode60FPS) {
        let target_fps = config.target_fps.unwrap_or(60) as f32;
        frame_scale = if config.cyspring_mono_uncap_frame_scale {
            (target_fps / 2.0).min(60.0)
        } else {
            60.0
        };
    }

    get_orig_fn!(UpdateForce, UpdateForceFn)(
        cloth_working, stiffness_force_rate, drag_force_rate,
        gravity_rate, wind_power, wind_strength, position_diff, frame_scale
    );
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, "Gallop", CySpringNative);

    // Resolve isNative static field before hooking cctor
    unsafe {
        IS_NATIVE_FIELD = get_field_from_name(CySpringNative, c"isNative");
    }

    // Hook .cctor to optionally force Mono path
    let cctor_addr = get_method_addr(CySpringNative, c".cctor", 0);
    if cctor_addr != 0 {
        new_hook!(cctor_addr, cctor);
    }

    let UpdateForce_addr = get_method_addr(CySpringNative, c"UpdateForce", 8);
    new_hook!(UpdateForce_addr, UpdateForce);
}
