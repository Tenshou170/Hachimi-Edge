use crate::{
    core::Hachimi,
    il2cpp::{symbols::{get_class_opt, get_method_addr}, types::*}
};

use super::CameraData::ShadowResolution;

pub fn shadow_resolution(orig_resolution: i32) -> i32 {
    let configured = Hachimi::instance().config.load().shadow_resolution;
    match configured {
        ShadowResolution::Default => orig_resolution,
        resolution => resolution as i32
    }
}

type CascadeShadow_GetShadowResolutionFn = extern "C" fn(this: *mut Il2CppObject, default_shadow_map_width: i32) -> i32;
extern "C" fn CascadeShadow_GetShadowResolution(this: *mut Il2CppObject, default_shadow_map_width: i32) -> i32 {
    let orig = get_orig_fn!(CascadeShadow_GetShadowResolution, CascadeShadow_GetShadowResolutionFn)(this, default_shadow_map_width);
    shadow_resolution(orig)
}

pub fn init(umamusume: *const Il2CppImage) {
    let Some(CascadeShadow) = get_class_opt(umamusume, c"Gallop.RenderPipeline", c"CascadeShadow") else {
        return;
    };

    let CascadeShadow_GetShadowResolution_addr = get_method_addr(CascadeShadow, c"GetShadowResolution", 1);
    new_hook!(CascadeShadow_GetShadowResolution_addr, CascadeShadow_GetShadowResolution);
}
