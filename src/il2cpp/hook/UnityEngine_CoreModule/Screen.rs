use crate::il2cpp::{api::il2cpp_resolve_icall, types::*};

#[cfg(target_os = "windows")]
use crate::{
    core::Hachimi,
    il2cpp::symbols::get_method_addr,
};

#[cfg(target_os = "windows")]
use crate::core::utils::scale_to_aspect_ratio;

#[cfg(target_os = "windows")]
static mut GET_CURRENTRESOLUTION_ADDR: usize = 0;
#[cfg(target_os = "windows")]
impl_addr_wrapper_fn!(get_currentResolution, GET_CURRENTRESOLUTION_ADDR, Resolution,);

#[cfg(target_os = "windows")]
static mut GET_WIDTH_ADDR: usize = 0;
#[cfg(target_os = "windows")]
impl_addr_wrapper_fn!(get_width, GET_WIDTH_ADDR, i32,);

#[cfg(target_os = "windows")]
static mut GET_HEIGHT_ADDR: usize = 0;
#[cfg(target_os = "windows")]
impl_addr_wrapper_fn!(get_height, GET_HEIGHT_ADDR, i32,);

#[cfg(target_os = "windows")]
static mut GET_FULLSCREEN_ADDR: usize = 0;
#[cfg(target_os = "windows")]
impl_addr_wrapper_fn!(get_fullScreen, GET_FULLSCREEN_ADDR, bool,);

#[cfg(target_os = "windows")]
pub fn apply_auto_full_screen(mut width: i32, mut height: i32) -> bool {
    let windows_config = &Hachimi::instance().config.load().windows;
    let preferred_res = &windows_config.full_screen_res;
    let (preferred_width, preferred_height) = if preferred_res.width > 0 && preferred_res.height > 0 {
        (preferred_res.width, preferred_res.height)
    }
    else {
        let res = get_currentResolution();
        (res.width, res.height)
    };

    if width > 0 && height > 0 && (width > height) == (preferred_width > preferred_height) {
        let aspect_ratio = width as f32 / height as f32;
        (width, height) = scale_to_aspect_ratio((preferred_width, preferred_height), aspect_ratio, false)
    }
    else {
        return false;
    }

    let full_screen_mode = windows_config.full_screen_mode as i32;
    let preferred_refresh_rate = RefreshRate {
        numerator: preferred_res.refresh_rate as u32,
        denominator: 1
    };
    get_orig_fn!(SetResolution_Injected, SetResolutionInjectedFn)(width, height, full_screen_mode, &preferred_refresh_rate);

    true
}

#[cfg(target_os = "windows")]
type SetResolutionInjectedFn = extern "C" fn(width: i32, height: i32, fullscreen_mode: i32, preferred_refresh_rate: *const RefreshRate);
#[cfg(target_os = "windows")]
extern "C" fn SetResolution_Injected(width: i32, height: i32, full_screen_mode: i32, preferred_refresh_rate: *const RefreshRate) {
    let windows_config = &Hachimi::instance().config.load().windows;
    if windows_config.auto_full_screen {
        if apply_auto_full_screen(width, height) {
            // Re-apply topmost after auto-fullscreen resolution change
            re_apply_topmost();
            return;
        }
    }

    get_orig_fn!(SetResolution_Injected, SetResolutionInjectedFn)(width, height, full_screen_mode, preferred_refresh_rate);

    // Re-apply topmost after any resolution/orientation change.
    // Windows resets the Z-order when the game transitions between portrait
    // and landscape (stories, lives, races), losing the "stay on top" state.
    re_apply_topmost();
}

#[cfg(target_os = "windows")]
fn re_apply_topmost() {
    let hachimi = Hachimi::instance();
    if hachimi.window_always_on_top.load(std::sync::atomic::Ordering::Relaxed) {
        let hwnd = crate::windows::wnd_hook::get_target_hwnd();
        if !hwnd.0.is_null() {
            unsafe { _ = crate::windows::utils::set_window_topmost(hwnd, true); }
        }
    }
}

static mut SET_SLEEPTIMEOUT_ADDR: usize = 0;
impl_addr_wrapper_fn!(set_sleepTimeout, SET_SLEEPTIMEOUT_ADDR, (), value: i32);

pub fn set_screen_timeout_disabled(disabled: bool) {
    let value = if disabled { -1 } else { -2 };
    unsafe {
        if SET_SLEEPTIMEOUT_ADDR != 0 {
            let set_timeout: extern "C" fn(i32) = std::mem::transmute(SET_SLEEPTIMEOUT_ADDR);
            set_timeout(value);
            info!("[Screen] sleepTimeout set to {} (disabled={})", value, disabled);
        } else {
            warn!("[Screen] set_sleepTimeout address is not resolved, cannot update sleepTimeout");
        }
    }
}

#[cfg(target_os = "windows")]
type GetWidthFn = extern "C" fn() -> i32;
#[cfg(target_os = "windows")]
extern "C" fn get_Width() -> i32 {
    if let Some((width, _)) = crate::windows::utils::get_scaling_res() {
        return width;
    }

    get_orig_fn!(get_Width, GetWidthFn)()
}

#[cfg(target_os = "windows")]
pub fn get_Width_orig() -> i32 {
    get_orig_fn!(get_Width, GetWidthFn)()
}

#[cfg(target_os = "windows")]
type GetHeightFn = extern "C" fn() -> i32;
#[cfg(target_os = "windows")]
extern "C" fn get_Height() -> i32 {
    if let Some((_, height)) = crate::windows::utils::get_scaling_res() {
        return height;
    }

    get_orig_fn!(get_Height, GetHeightFn)()
}

#[cfg(target_os = "windows")]
pub fn get_Height_orig() -> i32 {
    get_orig_fn!(get_Height, GetHeightFn)()
}

pub fn init(UnityEngine_CoreModule: *const Il2CppImage) {
    get_class_or_return!(UnityEngine_CoreModule, UnityEngine, Screen);
    let _ = Screen;

    #[cfg(target_os = "windows")]
    {
        let SetResolution_Injected_addr = il2cpp_resolve_icall(
            c"UnityEngine.Screen::SetResolution_Injected(System.Int32,System.Int32,\
            UnityEngine.FullScreenMode,UnityEngine.RefreshRate)".as_ptr()
        );

        new_hook!(SetResolution_Injected_addr, SetResolution_Injected);

        let get_Width_addr = get_method_addr(Screen, c"get_Width", 0);
        let get_Height_addr = get_method_addr(Screen, c"get_Height", 0);

        new_hook!(get_Width_addr, get_Width);
        new_hook!(get_Height_addr, get_Height);
    }

    unsafe {
        #[cfg(target_os = "windows")]
        {
            GET_CURRENTRESOLUTION_ADDR = get_method_addr(Screen, c"get_currentResolution", 0);
            GET_WIDTH_ADDR = il2cpp_resolve_icall(c"UnityEngine.Screen::get_width()".as_ptr());
            GET_HEIGHT_ADDR = il2cpp_resolve_icall(c"UnityEngine.Screen::get_height()".as_ptr());
            GET_FULLSCREEN_ADDR =il2cpp_resolve_icall(c"UnityEngine.Screen::get_fullScreen()".as_ptr());
        }

        SET_SLEEPTIMEOUT_ADDR = il2cpp_resolve_icall(c"UnityEngine.Screen::set_sleepTimeout(System.Int32)".as_ptr());
        if SET_SLEEPTIMEOUT_ADDR == 0 {
            SET_SLEEPTIMEOUT_ADDR = il2cpp_resolve_icall(c"UnityEngine.Screen::set_sleepTimeout".as_ptr());
        }
    }
}