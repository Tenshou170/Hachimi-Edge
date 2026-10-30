use crate::{
    il2cpp::{symbols::{get_class_opt, get_method_addr}, types::*}
};
use super::CascadeShadow;

type CascadeShadowForRace_GetShadowResolutionFn = extern "C" fn(this: *mut Il2CppObject, default_shadow_map_width: i32) -> i32;
extern "C" fn CascadeShadowForRace_GetShadowResolution(this: *mut Il2CppObject, default_shadow_map_width: i32) -> i32 {
    let orig = get_orig_fn!(CascadeShadowForRace_GetShadowResolution, CascadeShadowForRace_GetShadowResolutionFn)(this, default_shadow_map_width);
    CascadeShadow::shadow_resolution(orig)
}

pub fn init(umamusume: *const Il2CppImage) {
    let Some(CascadeShadowForRace) = get_class_opt(umamusume, c"Gallop.RenderPipeline", c"CascadeShadowForRace") else {
        return;
    };

    let CascadeShadowForRace_GetShadowResolution_addr = get_method_addr(CascadeShadowForRace, c"GetShadowResolution", 1);
    new_hook!(CascadeShadowForRace_GetShadowResolution_addr, CascadeShadowForRace_GetShadowResolution);
}
