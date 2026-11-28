use crate::il2cpp::{api::il2cpp_resolve_icall, symbols::{get_class_opt, get_method_addr_opt}, types::*};

static mut GET_GRAPHICSMEMORYSIZE_ADDR: usize = 0;
static mut GET_MAXTEXTURESIZE_ADDR: usize = 0;

pub fn graphics_memory_size_mb() -> i32 {
    unsafe {
        if GET_GRAPHICSMEMORYSIZE_ADDR == 0 {
            return 0;
        }
        let f: extern "C" fn() -> i32 = std::mem::transmute(GET_GRAPHICSMEMORYSIZE_ADDR);
        f()
    }
}

pub fn max_texture_size() -> i32 {
    unsafe {
        if GET_MAXTEXTURESIZE_ADDR == 0 {
            return 0;
        }
        let f: extern "C" fn() -> i32 = std::mem::transmute(GET_MAXTEXTURESIZE_ADDR);
        f()
    }
}

pub fn init(UnityEngine_CoreModule: *const Il2CppImage) {
    let mut mem_addr = il2cpp_resolve_icall(
        c"UnityEngine.SystemInfo::get_graphicsMemorySize()".as_ptr()
    );
    let mut tex_addr = il2cpp_resolve_icall(
        c"UnityEngine.SystemInfo::get_maxTextureSize()".as_ptr()
    );

    if mem_addr == 0 || tex_addr == 0 {
        if let Some(SystemInfo) = get_class_opt(UnityEngine_CoreModule, c"UnityEngine", c"SystemInfo") {
            if mem_addr == 0 {
                mem_addr = get_method_addr_opt(SystemInfo, c"get_graphicsMemorySize", 0).unwrap_or(0);
            }
            if tex_addr == 0 {
                tex_addr = get_method_addr_opt(SystemInfo, c"get_maxTextureSize", 0).unwrap_or(0);
            }
        }
    }

    unsafe {
        GET_GRAPHICSMEMORYSIZE_ADDR = mem_addr;
        GET_MAXTEXTURESIZE_ADDR = tex_addr;
    }
}
