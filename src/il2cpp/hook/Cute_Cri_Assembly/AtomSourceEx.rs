use crate::{
    core::live_utils::CriAtomExPlayback,
    il2cpp::{
        symbols::get_method_addr,
        types::*
    }
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

static mut SET_PLAYBACK_ADDR: usize = 0;

pub fn set_Playback(this: *mut Il2CppObject, value: CriAtomExPlayback) {
    let addr = unsafe { SET_PLAYBACK_ADDR };
    if addr == 0 { return; }
    let orig_fn: extern "C" fn(*mut Il2CppObject, CriAtomExPlayback) =
        unsafe { std::mem::transmute(addr) };
    orig_fn(this, value)
}

pub fn init(Cute_Cri_Assembly: *const Il2CppImage) {
    get_class_or_return!(Cute_Cri_Assembly, "Cute.Cri", AtomSourceEx);

    unsafe {
        CLASS = AtomSourceEx;
        GET_PLAYER_ADDR   = get_method_addr(AtomSourceEx, c"get_player", 0);
        SET_PLAYBACK_ADDR = get_method_addr(AtomSourceEx, c"set_Playback", 1);
    }
}
