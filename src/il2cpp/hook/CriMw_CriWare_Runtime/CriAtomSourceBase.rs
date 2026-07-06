use crate::il2cpp::{
    symbols::get_method_addr,
    types::*
};

static mut CLASS: *mut Il2CppClass = 0 as _;

pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

static mut GET_PLAYER_ADDR: usize = 0;

pub fn get_player(this: *mut Il2CppObject) -> *mut Il2CppObject {
    let addr = unsafe { GET_PLAYER_ADDR };
    if addr == 0 { return std::ptr::null_mut(); }
    let orig_fn: extern "C" fn(*mut Il2CppObject) -> *mut Il2CppObject =
        unsafe { std::mem::transmute(addr) };
    orig_fn(this)
}

pub fn init(CriMw_CriWare_Runtime: *const Il2CppImage) {
    get_class_or_return!(CriMw_CriWare_Runtime, CriWare, CriAtomSourceBase);

    unsafe {
        CLASS = CriAtomSourceBase;
        GET_PLAYER_ADDR = get_method_addr(CriAtomSourceBase, c"get_player", 0);
    }
}
