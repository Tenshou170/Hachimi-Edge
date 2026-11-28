use crate::{
    core::Hachimi,
    il2cpp::{symbols::{get_class_opt, get_field_from_name, get_method_addr}, types::*}
};

use super::CameraData::ShadowResolution;

def_field_value_accessors!(set set_m_ShadowmapWidth, SHADOWMAPWIDTH_FIELD, i32);
def_field_value_accessors!(set set_m_ShadowmapHeight, SHADOWMAPHEIGHT_FIELD, i32);
def_field_value_accessors!(get_m_ShadowCasterCascadesCount, _set_m_ShadowCasterCascadesCount, SHADOWCASTERCASCADESCOUNT_FIELD, i32);

type SetupFn = extern "C" fn(this: *mut Il2CppObject, rendering_data: *mut Il2CppObject) -> bool;
extern "C" fn Setup(this: *mut Il2CppObject, rendering_data: *mut Il2CppObject) -> bool {
    let ok = get_orig_fn!(Setup, SetupFn)(this, rendering_data);
    if ok {
        let configured = Hachimi::instance().config.load().shadow_resolution;
        if configured != ShadowResolution::Default {
            let res = configured as i32;
            unsafe {
                if !SHADOWMAPWIDTH_FIELD.is_null() && !SHADOWMAPHEIGHT_FIELD.is_null() {
                    set_m_ShadowmapWidth(this, res);
                    let cascades = if !SHADOWCASTERCASCADESCOUNT_FIELD.is_null() {
                        get_m_ShadowCasterCascadesCount(this)
                    } else {
                        1
                    };
                    let height = if cascades == 2 { res / 2 } else { res };
                    set_m_ShadowmapHeight(this, height);
                }
            }
        }
    }
    ok
}

pub fn init(umamusume: *const Il2CppImage) {
    let Some(MainLightShadowCasterPass) = get_class_opt(umamusume, c"Gallop.RenderPipeline", c"MainLightShadowCasterPass") else {
        return;
    };

    unsafe {
        SHADOWMAPWIDTH_FIELD = get_field_from_name(MainLightShadowCasterPass, c"m_ShadowmapWidth");
        SHADOWMAPHEIGHT_FIELD = get_field_from_name(MainLightShadowCasterPass, c"m_ShadowmapHeight");
        SHADOWCASTERCASCADESCOUNT_FIELD = get_field_from_name(MainLightShadowCasterPass, c"m_ShadowCasterCascadesCount");
    }

    let Setup_addr = get_method_addr(MainLightShadowCasterPass, c"Setup", 1);
    new_hook!(Setup_addr, Setup);
}
