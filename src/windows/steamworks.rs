#![allow(non_snake_case, non_upper_case_globals)]

use std::{os::raw::c_void, ptr::NonNull, sync::atomic::{AtomicUsize, Ordering}};

use windows::Win32::Foundation::HMODULE;

use crate::{core::Hachimi, windows::utils};

// --- W-6 fix: replace static mut with AtomicUsize so reads from any thread
// are race-free. Written once in init() with Release, read with Relaxed. ---
static SteamAPI_SteamUtils_v010_addr: AtomicUsize = AtomicUsize::new(0);
static SteamAPI_ISteamUtils_IsOverlayEnabled_addr: AtomicUsize = AtomicUsize::new(0);

#[repr(transparent)]
pub struct SteamUtils(NonNull<c_void>);

impl SteamUtils {
    pub fn get() -> Option<SteamUtils> {
        let addr = SteamAPI_SteamUtils_v010_addr.load(Ordering::Relaxed);
        if addr == 0 {
            return None;
        }

        let orig_fn: extern "C" fn() -> *mut c_void = unsafe { std::mem::transmute(addr) };
        NonNull::new(orig_fn()).map(|p| Self(p))
    }

    pub fn is_overlay_enabled(&self) -> bool {
        let addr = SteamAPI_ISteamUtils_IsOverlayEnabled_addr.load(Ordering::Relaxed);
        if addr == 0 { return false; }
        let orig_fn: extern "C" fn(*mut c_void) -> bool = unsafe { std::mem::transmute(addr) };
        orig_fn(self.0.as_ptr())
    }
}

pub fn init(steam_api: HMODULE) {
    let utils_addr = utils::get_proc_address(steam_api, c"SteamAPI_SteamUtils_v010");
    let overlay_addr = utils::get_proc_address(steam_api, c"SteamAPI_ISteamUtils_IsOverlayEnabled");
    SteamAPI_SteamUtils_v010_addr.store(utils_addr, Ordering::Release);
    SteamAPI_ISteamUtils_IsOverlayEnabled_addr.store(overlay_addr, Ordering::Release);
}

fn is_using_overlay() -> bool {
    std::env::var("SteamOverlayGameId").is_ok()
}

pub fn is_overlay_conflicting(hachimi: &Hachimi) -> bool {
    if SteamUtils::get().is_some_and(|u| u.is_overlay_enabled()) {
        // overlay has successfully initialized and is not conflicting with Hachimi
        return false;
    }

    if !hachimi.game.is_steam_release || hachimi.config.load().disable_gui {
        return false;
    }

    is_using_overlay()
}