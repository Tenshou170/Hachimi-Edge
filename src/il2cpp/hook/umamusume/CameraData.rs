use std::ptr::null_mut;
use serde::{Serialize, Deserialize};
use crate::{
    core::{capabilities::{self, Capability}, Hachimi},
    il2cpp::{symbols::{get_class_opt, get_field_from_name, get_method_addr}, types::*}
};

#[derive(Default, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[repr(i32)]
pub enum ShadowResolution {
    #[default] Default,
    _256 = 0x100,
    _512 = 0x200,
    _1024 = 0x400,
    _2048 = 0x800,
    _4096 = 0x1000
}

static mut CLASS: *mut Il2CppClass = null_mut();
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

static mut SET_ISOVERRIDESHADOWRESOLUTION_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_IsOverrideShadowResolution, SET_ISOVERRIDESHADOWRESOLUTION_ADDR, (), this: *mut Il2CppObject, value: bool);

static mut SET_OVERRIDESHADOWRESOLUTION_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_OverrideShadowResolution, SET_OVERRIDESHADOWRESOLUTION_ADDR, (), this: *mut Il2CppObject, value: ShadowResolution);

def_field_value_accessors!(get_MaxShadowDistanceField, set_MaxShadowDistanceField, MAXSHADOWDISTANCE_FIELD, f32);

pub fn raise_shadow_distance_to_floor(value: f32, floor: f32) -> f32 {
    if floor > 0.0 && value > 0.0 && value < floor {
        floor
    } else {
        value
    }
}

type SetMaxShadowDistanceFn = extern "C" fn(this: *mut Il2CppObject, value: f32);
extern "C" fn set_MaxShadowDistance(this: *mut Il2CppObject, value: f32) {
    let floor = Hachimi::instance().config.load().shadow_distance;
    let value = raise_shadow_distance_to_floor(value, floor);
    get_orig_fn!(set_MaxShadowDistance, SetMaxShadowDistanceFn)(this, value)
}

type InitializeFn = extern "C" fn(this: *mut Il2CppObject, renderer_index: i32);
extern "C" fn Initialize(this: *mut Il2CppObject, renderer_index: i32) {
    get_orig_fn!(Initialize, InitializeFn)(this, renderer_index);
    if unsafe { !MAXSHADOWDISTANCE_FIELD.is_null() } {
        let floor = Hachimi::instance().config.load().shadow_distance;
        let current = get_MaxShadowDistanceField(this);
        let raised = raise_shadow_distance_to_floor(current, floor);
        if raised != current {
            set_MaxShadowDistanceField(this, raised);
        }
    }
}

pub fn init(umamusume: *const Il2CppImage) {
    // `Gallop.RenderPipeline` does not exist on every client (the Global build
    // renders through a different pipeline), so probe for it instead of
    // hard-coding a region check: if a client ships it later, the shadow
    // options light up on their own.
    let Some(CameraData) = get_class_opt(umamusume, c"Gallop.RenderPipeline", c"CameraData") else {
        return;
    };

    unsafe {
        CLASS = CameraData;
        SET_ISOVERRIDESHADOWRESOLUTION_ADDR = get_method_addr(CameraData, c"set_IsOverrideShadowResolution", 1);
        SET_OVERRIDESHADOWRESOLUTION_ADDR = get_method_addr(CameraData, c"set_OverrideShadowResolution", 1);
        MAXSHADOWDISTANCE_FIELD = get_field_from_name(CameraData, c"<MaxShadowDistance>k__BackingField");
    }

    let SetMaxShadowDistance_addr = get_method_addr(CameraData, c"set_MaxShadowDistance", 1);
    new_hook!(SetMaxShadowDistance_addr, set_MaxShadowDistance);

    let Initialize_addr = get_method_addr(CameraData, c"Initialize", 1);
    new_hook!(Initialize_addr, Initialize);

    // The shadow options only make sense when the whole pipeline is present:
    // resolution lives here, per-cascade resolution in CascadeShadow, and the
    // distance/soft-shadow/bias overrides in GallopRenderer.
    let pipeline_complete = ["CascadeShadow", "CascadeShadowForRace", "GallopRenderer"].iter().all(|name| {
        let name = std::ffi::CString::new(*name).unwrap();
        get_class_opt(umamusume, c"Gallop.RenderPipeline", &name).is_some()
    });
    if pipeline_complete {
        capabilities::set(Capability::UrpShadowPipeline);
    }
}
