use crate::il2cpp::{
    symbols::{get_method_addr, get_type_object_for_class},
    types::*
};

static mut CLASS: *mut Il2CppClass = 0 as _;
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

static mut TYPE_OBJECT: *mut Il2CppObject = 0 as _;
pub fn type_object() -> *mut Il2CppObject {
    unsafe { TYPE_OBJECT }
}

static mut GET_ISPLAYING_ADDR: usize = 0;
pub fn get_IsPlaying(this: *mut Il2CppObject) -> bool {
    let addr = unsafe { GET_ISPLAYING_ADDR };
    if addr == 0 || this.is_null() { return false; }
    let orig_fn: extern "C" fn(*mut Il2CppObject) -> bool =
        unsafe { std::mem::transmute(addr) };
    orig_fn(this)
}

static mut IS_OPEN_ADDR: usize = 0;
pub fn IsOpen(this: *mut Il2CppObject) -> bool {
    let addr = unsafe { IS_OPEN_ADDR };
    if addr == 0 || this.is_null() { return false; }
    let orig_fn: extern "C" fn(*mut Il2CppObject) -> bool =
        unsafe { std::mem::transmute(addr) };
    orig_fn(this)
}

pub fn is_active_or_playing(this: *mut Il2CppObject) -> bool {
    if this.is_null() { return false; }
    IsOpen(this) || get_IsPlaying(this)
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, PartsCharaMessageBase);

    unsafe {
        CLASS = PartsCharaMessageBase;
        TYPE_OBJECT = get_type_object_for_class(PartsCharaMessageBase);
        GET_ISPLAYING_ADDR = get_method_addr(PartsCharaMessageBase, c"get_IsPlaying", 0);
        IS_OPEN_ADDR = get_method_addr(PartsCharaMessageBase, c"IsOpen", 0);
    }
}
