#![allow(non_snake_case, non_upper_case_globals)]

use std::path::PathBuf;
use widestring::U16CString;
use windows::{core::PCWSTR, Win32::System::LibraryLoader::LoadLibraryW};

use crate::{core::Hachimi, windows::utils};

proxy_proc!(criVvp9_GetAlphaInterface, criVvp9_GetAlphaInterface_orig);
proxy_proc!(criVvp9_GetInterface, criVvp9_GetInterface_orig);
proxy_proc!(criVvp9_SetUserAllocator, criVvp9_SetUserAllocator_orig);

fn find_real_dll() -> Option<PathBuf> {
    let our_path = unsafe {
        if !crate::windows::main::DLL_HMODULE.is_invalid() {
            utils::get_module_path(crate::windows::main::DLL_HMODULE)
        } else {
            PathBuf::new()
        }
    };
    let our_canonical = our_path.canonicalize().ok();

    let is_same = |p: &std::path::Path| -> bool {
        if let Some(ref our) = our_canonical {
            if let Ok(canon) = p.canonicalize() {
                return &canon == our;
            }
        }
        false
    };

    let game_dir = utils::get_game_dir();

    // 1. Direct candidates in hachimi dir or game dir
    let candidates = [
        Hachimi::instance().get_data_path("cri_mana_vpx.dll"),
        Hachimi::instance().get_data_path("cri_mana_vpx_orig.dll"),
        game_dir.join("cri_mana_vpx_orig.dll"),
    ];
    for c in candidates {
        if c.is_file() && !is_same(&c) {
            return Some(c);
        }
    }

    // 2. Scan for Unity plugins directory: <game_dir>/*_Data/Plugins/x86_64/cri_mana_vpx.dll
    // or cri_mana_vpx_orig.dll
    if let Ok(entries) = std::fs::read_dir(&game_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && path.file_name().map(|n| n.to_string_lossy().ends_with("_Data")).unwrap_or(false) {
                let orig = path.join("Plugins").join("x86_64").join("cri_mana_vpx_orig.dll");
                if orig.is_file() && !is_same(&orig) {
                    return Some(orig);
                }
                let real = path.join("Plugins").join("x86_64").join("cri_mana_vpx.dll");
                if real.is_file() && !is_same(&real) {
                    return Some(real);
                }
            }
        }
    }

    None
}

pub fn init() {
    let Some(dll_path) = find_real_dll() else {
        warn!("cri_mana_vpx.dll real library not found, skipping");
        return;
    };

    info!("Loading real cri_mana_vpx library from: {}", dll_path.display());

    unsafe {
        let dll_path_cstr = match U16CString::from_str(dll_path.to_str().unwrap_or("")) {
            Ok(s) => s,
            Err(e) => {
                error!("[cri_mana_vpx] Failed to encode DLL path: {}", e);
                return;
            }
        };
        let handle = match LoadLibraryW(PCWSTR(dll_path_cstr.as_ptr())) {
            Ok(h) => h,
            Err(e) => {
                error!("[cri_mana_vpx] Failed to load real cri_mana_vpx library: {}", e);
                return;
            }
        };

        criVvp9_GetAlphaInterface_orig = utils::get_proc_address(handle, c"criVvp9_GetAlphaInterface");
        criVvp9_GetInterface_orig = utils::get_proc_address(handle, c"criVvp9_GetInterface");
        criVvp9_SetUserAllocator_orig = utils::get_proc_address(handle, c"criVvp9_SetUserAllocator");
    }
}