#![allow(non_snake_case)]

use std::path::Path;

use windows::{core::{w, PCWSTR}, Win32::{Foundation::HMODULE, System::LibraryLoader::GetModuleHandleW}};

use crate::{core::{Error, Hachimi}, windows::{steamworks, utils}};

use super::{hachimi_impl, proxy, ffi};

type LoadLibraryWFn = extern "C" fn(filename: PCWSTR) -> HMODULE;
extern "C" fn LoadLibraryW(filename: PCWSTR) -> HMODULE {
    let hachimi = Hachimi::instance();
    // --- W-10 fix: get_trampoline_addr returns 0 if the hook isn't installed
    // yet.  Guard before transmute so we never call a null function pointer. ---
    let trampoline = hachimi.interceptor.get_trampoline_addr(LoadLibraryW as *const () as usize);
    if trampoline == 0 {
        // Hook not yet active — fall back to the real LoadLibraryW via FFI
        return unsafe { crate::windows::ffi::LoadLibraryW(filename) };
    }
    let orig_fn: LoadLibraryWFn = unsafe { std::mem::transmute(trampoline) };

    let handle = orig_fn(filename);
    // --- W-5 fix: to_string() can fail on invalid UTF-16 (Wine path
    // translation artifacts, etc.).  Treat decode failure as "not our DLL"
    // and pass through rather than panicking inside the hook. ---
    let filename_str = match unsafe { filename.to_string() } {
        Ok(s) => s,
        Err(_) => return handle,
    };

    if hachimi_impl::is_criware_lib(&filename_str) {
        // Manually trigger a GameAssembly.dll load anyways since hachimi might have been loaded later
        let assembly_module = orig_fn(w!("GameAssembly.dll")).0 as usize;
        if assembly_module != 0 {
            hachimi.on_dlopen("GameAssembly.dll", assembly_module);
        }
    }

    let needs_init_steamworks = steamworks::is_overlay_conflicting(&hachimi);
    if hachimi.on_dlopen(&filename_str, handle.0 as usize) {
        if !needs_init_steamworks {
            hachimi.interceptor.unhook(LoadLibraryW as *const () as usize);
        }
    }
    else if needs_init_steamworks &&
        Path::new(&filename_str).file_name().is_some_and(|name| name == "steam_api64.dll")
    {
        steamworks::init(handle);
        hachimi.interceptor.unhook(LoadLibraryW as *const () as usize);
    }
    handle
}

fn init_internal() -> Result<(), Error> {
    let hachimi = Hachimi::instance();
    if let Ok(handle) = unsafe { GetModuleHandleW(w!("GameAssembly.dll")) } {
        info!("Late loading detected");

        info!("Hooking LoadLibraryW");
        hachimi.interceptor.hook(ffi::LoadLibraryW as *const () as usize, LoadLibraryW as *const () as usize)?;

        info!("Init cri_mana_vpx.dll proxy");
        proxy::cri_mana_vpx::init();

        hachimi.on_dlopen("GameAssembly.dll", handle.0 as _);
        hachimi.on_hooking_finished();   
    }
    else {
        info!("Init UnityPlayer.dll proxy");
        proxy::unityplayer::init();

        let system_dir = utils::_get_system_directory();

        info!("Init winhttp.dll proxy");
        proxy::winhttp::init(&system_dir);

        info!("Hooking LoadLibraryW");
        hachimi.interceptor.hook(ffi::LoadLibraryW as *const () as usize, LoadLibraryW as *const () as usize)?;
    }

    Ok(())
}

pub fn init() {
    init_internal().unwrap_or_else(|e| {
        error!("Init failed: {}", e);
    });
}