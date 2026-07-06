use crate::{
    core::{captions, live_utils},
    il2cpp::{
        symbols::{get_method_addr, get_method_overload_addr},
        types::*
    }
};

static mut START_ADDR: usize = 0;
pub fn Start(this: *mut Il2CppObject) -> live_utils::CriAtomExPlayback {
    let addr = unsafe { START_ADDR };
    if addr == 0 { return live_utils::CriAtomExPlayback { id: 0 }; }
    let orig_fn: extern "C" fn(*mut Il2CppObject) -> live_utils::CriAtomExPlayback =
        unsafe { std::mem::transmute(addr) };
    orig_fn(this)
}

static mut STOP_ADDR: usize = 0;
pub fn Stop(this: *mut Il2CppObject, ignores_release_time: bool) {
    let addr = unsafe { STOP_ADDR };
    if addr == 0 { return; }
    let orig_fn: extern "C" fn(*mut Il2CppObject, bool) =
        unsafe { std::mem::transmute(addr) };
    orig_fn(this, ignores_release_time)
}

static mut STOPWITHOUTRELEASETIME_ADDR: usize = 0;
pub fn StopWithoutReleaseTime(this: *mut Il2CppObject) {
    let addr = unsafe { STOPWITHOUTRELEASETIME_ADDR };
    if addr == 0 { return; }
    let orig_fn: extern "C" fn(*mut Il2CppObject) =
        unsafe { std::mem::transmute(addr) };
    orig_fn(this)
}

static mut SETSTARTTIME_ADDR: usize = 0;
pub fn SetStartTime(this: *mut Il2CppObject, start_time_ms: i64) {
    let addr = unsafe { SETSTARTTIME_ADDR };
    if addr == 0 { return; }
    let orig_fn: extern "C" fn(*mut Il2CppObject, i64) =
        unsafe { std::mem::transmute(addr) };
    orig_fn(this, start_time_ms)
}

static mut UPDATE_ADDR: usize = 0;
pub fn Update(this: *mut Il2CppObject, playback: live_utils::CriAtomExPlayback) {
    let addr = unsafe { UPDATE_ADDR };
    if addr == 0 { return; }
    let orig_fn: extern "C" fn(*mut Il2CppObject, live_utils::CriAtomExPlayback) =
        unsafe { std::mem::transmute(addr) };
    orig_fn(this, playback)
}

static mut PAUSE_ADDR: usize = 0;
pub fn Pause(this: *mut Il2CppObject, sw: bool) {
    let addr = unsafe { PAUSE_ADDR };
    if addr == 0 { return; }
    let orig_fn: extern "C" fn(*mut Il2CppObject, bool) =
        unsafe { std::mem::transmute(addr) };
    orig_fn(this, sw)
}

// ── Hooks ────────────────────────────────────────────────────────────────────

// public Void Stop()
type StopHookFn = extern "C" fn(this: *mut Il2CppObject);
pub extern "C" fn StopHook(this: *mut Il2CppObject) {
    get_orig_fn!(StopHook, StopHookFn)(this);
    captions::Captions::cleanup();
}

// public void StopWithoutReleaseTime()
pub type StopWithoutReleaseTimeHookFn = extern "C" fn(this: *mut Il2CppObject);
pub extern "C" fn StopWithoutReleaseTimeHook(this: *mut Il2CppObject) {
    get_orig_fn!(StopWithoutReleaseTimeHook, StopWithoutReleaseTimeHookFn)(this);
    captions::Captions::cleanup();
}

// public Void Pause(Boolean sw)
type PauseHookFn = extern "C" fn(this: *mut Il2CppObject, sw: bool);
pub extern "C" fn PauseHook(this: *mut Il2CppObject, sw: bool) {
    get_orig_fn!(PauseHook, PauseHookFn)(this, sw);
    if !sw {
        captions::Captions::cleanup();
    }
}

pub fn init(CriMw_CriWare_Runtime: *const Il2CppImage) {
    get_class_or_return!(CriMw_CriWare_Runtime, CriWare, CriAtomExPlayer);

    unsafe {
        STOP_ADDR                   = get_method_addr(CriAtomExPlayer, c"Stop", 1);
        STOPWITHOUTRELEASETIME_ADDR = get_method_addr(CriAtomExPlayer, c"StopWithoutReleaseTime", 0);
        START_ADDR                  = get_method_addr(CriAtomExPlayer, c"Start", 0);
        PAUSE_ADDR                  = get_method_addr(CriAtomExPlayer, c"Pause", 1);
        SETSTARTTIME_ADDR           = get_method_addr(CriAtomExPlayer, c"SetStartTime", 1);
        UPDATE_ADDR                 = get_method_addr(CriAtomExPlayer, c"Update", 1);
    }

    let stop_addr = get_method_addr(CriAtomExPlayer, c"Stop", 0);
    new_hook!(stop_addr, StopHook);

    let stop_without_release_time_addr = get_method_addr(CriAtomExPlayer, c"StopWithoutReleaseTime", 0);
    new_hook!(stop_without_release_time_addr, StopWithoutReleaseTimeHook);

    let pause_addr = get_method_overload_addr(CriAtomExPlayer, "Pause", &[Il2CppTypeEnum_IL2CPP_TYPE_BOOLEAN]);
    new_hook!(pause_addr, PauseHook);
}
