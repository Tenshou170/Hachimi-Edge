use crate::{
    il2cpp::{
        hook::Unity_RenderPipelines_Universal_Runtime::UniversalRenderPipelineAsset,
        symbols::{get_class_opt, get_method_addr},
        types::*
    }
};

type SetupFn = extern "C" fn(this: *mut Il2CppObject, context: usize, rendering_data: *mut Il2CppObject);
extern "C" fn Setup(this: *mut Il2CppObject, context: usize, rendering_data: *mut Il2CppObject) {
    UniversalRenderPipelineAsset::apply_shadow_overrides();
    get_orig_fn!(Setup, SetupFn)(this, context, rendering_data);
}

pub fn init(umamusume: *const Il2CppImage) {
    let Some(GallopRenderer) = get_class_opt(umamusume, c"Gallop.RenderPipeline", c"GallopRenderer") else {
        return;
    };

    let Setup_addr = get_method_addr(GallopRenderer, c"Setup", 2);
    new_hook!(Setup_addr, Setup);
}
