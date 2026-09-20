use std::sync::atomic;

use crate::{core::Hachimi, il2cpp::{api::il2cpp_resolve_icall, symbols::get_method_addr, types::*}};

type SetTargetFrameRateFn = extern "C" fn(value: i32);
pub extern "C" fn set_targetFrameRate(mut value: i32) {
    // Remember the game's own requested FPS so poke_target_frame_rate can restore it.
    #[cfg(target_os = "windows")]
    LAST_GAME_FPS.store(value, atomic::Ordering::Relaxed);

    let hachimi = Hachimi::instance();
    let target_fps = hachimi.target_fps.load(atomic::Ordering::Relaxed);
    if target_fps != -1 {
        value = target_fps.clamp(30, 240);
    }

    #[cfg(target_os = "windows")]
    {
        let unfocused_fps = hachimi.target_fps_unfocused.load(atomic::Ordering::Relaxed);
        if unfocused_fps != -1 && crate::windows::wnd_hook::window_unfocused() {
            value = unfocused_fps;
        }
    }

    get_orig_fn!(set_targetFrameRate, SetTargetFrameRateFn)(value);
}

/// Last FPS value the game itself requested (before our override).
/// Used by poke_target_frame_rate to restore the correct rate on focus-in.
#[cfg(target_os = "windows")]
static LAST_GAME_FPS: atomic::AtomicI32 = atomic::AtomicI32::new(-1);

/// Computes the frame rate that should currently be active, accounting for
/// target_fps override and target_fps_unfocused when the window is not focused.
#[cfg(target_os = "windows")]
pub fn current_effective_frame_rate() -> i32 {
    let hachimi = Hachimi::instance();
    let target_fps = hachimi.target_fps.load(atomic::Ordering::Relaxed);
    let unfocused_fps = hachimi.target_fps_unfocused.load(atomic::Ordering::Relaxed);
    if unfocused_fps != -1 && crate::windows::wnd_hook::window_unfocused() {
        unfocused_fps
    } else if target_fps != -1 {
        target_fps
    } else {
        LAST_GAME_FPS.load(atomic::Ordering::Relaxed)
    }
}

/// Schedules the game thread to re-apply the correct frame rate cap.
/// Called on WM_ACTIVATE focus changes so the unfocused cap kicks in/out immediately.
#[cfg(target_os = "windows")]
pub fn poke_target_frame_rate() {
    let value = current_effective_frame_rate();
    if value != -1 {
        set_targetFrameRate(value);
    }
}

static mut GET_PERSISTENTDATAPATH_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_persistentDataPath, GET_PERSISTENTDATAPATH_ADDR, *mut Il2CppString,);

static mut OPENURL_ADDR: usize = 0;
impl_addr_wrapper_fn!(OpenURL, OPENURL_ADDR, (), url: *mut Il2CppString);

static mut GET_SYSTEMLANGUAGE_ADDR: usize = 0;
impl_addr_wrapper_fn!(systemLanguage, GET_SYSTEMLANGUAGE_ADDR, i32, );

pub fn init(UnityEngine_CoreModule: *const Il2CppImage) {
    get_class_or_return!(UnityEngine_CoreModule, UnityEngine, Application);

    let set_targetFrameRate_addr = il2cpp_resolve_icall(
        c"UnityEngine.Application::set_targetFrameRate(System.Int32)".as_ptr()
    );
    new_hook!(set_targetFrameRate_addr, set_targetFrameRate);

    unsafe {
        GET_PERSISTENTDATAPATH_ADDR = get_method_addr(Application, c"get_persistentDataPath", 0);
        OPENURL_ADDR = get_method_addr(Application, c"OpenURL", 1);
        GET_SYSTEMLANGUAGE_ADDR = il2cpp_resolve_icall(c"UnityEngine.Application::get_systemLanguage()".as_ptr());
    }
}
