use crate::il2cpp::{api::il2cpp_resolve_icall, symbols::get_method_addr, types::*};

static mut GET_ENABLED_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_enabled, GET_ENABLED_ADDR, bool, this: *mut Il2CppObject);

static mut SET_ENABLED_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_enabled, SET_ENABLED_ADDR, (), this: *mut Il2CppObject, value: bool);

static mut GET_IS_ACTIVE_AND_ENABLED_ADDR: usize = 0;
pub fn get_isActiveAndEnabled(this: *mut Il2CppObject) -> bool {
    let addr = unsafe { GET_IS_ACTIVE_AND_ENABLED_ADDR };
    if addr == 0 || this.is_null() {
        return get_enabled(this);
    }
    let orig_fn: extern "C" fn(this: *mut Il2CppObject) -> bool = unsafe { std::mem::transmute(addr) };
    orig_fn(this)
}

pub fn init(UnityEngine_CoreModule: *const Il2CppImage) {
    get_class_or_return!(UnityEngine_CoreModule, UnityEngine, Behaviour);
    unsafe {
        GET_ENABLED_ADDR = il2cpp_resolve_icall(c"UnityEngine.Behaviour::get_enabled()".as_ptr());
        SET_ENABLED_ADDR = il2cpp_resolve_icall(c"UnityEngine.Behaviour::set_enabled(System.Boolean)".as_ptr());
        GET_IS_ACTIVE_AND_ENABLED_ADDR = get_method_addr(Behaviour, c"get_isActiveAndEnabled", 0);
    }
}