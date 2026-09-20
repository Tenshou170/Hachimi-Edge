use std::sync::atomic;

use serde::{Deserialize, Serialize};

use crate::{
    core::Hachimi,
    il2cpp::{
        hook::UnityEngine_CoreModule::{
            FullScreenMode_ExclusiveFullScreen, FullScreenMode_FullScreenWindow,
            QualitySettings, Screen
        }, symbols::Thread, types::Resolution
    }
};

use super::{utils, wnd_hook};

pub fn is_il2cpp_lib(filename: &str) -> bool {
    std::path::Path::new(filename)
        .file_name()
        .is_some_and(|name| name.eq_ignore_ascii_case("GameAssembly.dll"))
}

pub fn is_criware_lib(filename: &str) -> bool {
    std::path::Path::new(filename)
        .file_name()
        .is_some_and(|name| name.eq_ignore_ascii_case("cri_ware_unity.dll"))
}

/// Detect Wine/Proton.
///
/// Primary check: the `wine_get_version` export in ntdll.dll — under native
/// Windows this symbol does not exist. Some Wine builds (e.g. launched with
/// `WINEDLLOVERRIDES=ntdll=n`) hide that export, so fall back to scanning
/// ntdll's version resource for a Wine marker string.
pub fn is_wine() -> bool {
    use windows::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};
    use windows::core::PCSTR;
    unsafe {
        let ntdll = GetModuleHandleA(PCSTR(b"ntdll.dll\0".as_ptr()));
        if let Ok(h) = ntdll {
            if GetProcAddress(h, PCSTR(b"wine_get_version\0".as_ptr())).is_some() {
                return true;
            }
        }
    }
    ntdll_version_mentions_wine()
}

/// Fallback detection: Wine's ntdll version resource names "Wine" / "WineHQ"
/// in its string table. Query each candidate field for the marker.
fn ntdll_version_mentions_wine() -> bool {
    use std::ffi::c_void;
    use windows::core::{w, PCWSTR};
    use windows::Win32::{
        Foundation::MAX_PATH,
        Storage::FileSystem::{GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW},
        System::LibraryLoader::{GetModuleFileNameW, GetModuleHandleW},
    };

    unsafe {
        let Ok(ntdll) = GetModuleHandleW(w!("ntdll.dll")) else {
            return false;
        };

        let mut path_buf = [0u16; MAX_PATH as usize];
        let len = GetModuleFileNameW(Some(ntdll), &mut path_buf);
        if len == 0 {
            return false;
        }

        let mut handle = 0u32;
        let size = GetFileVersionInfoSizeW(PCWSTR(path_buf.as_ptr()), Some(&mut handle));
        if size == 0 {
            return false;
        }

        let mut data = vec![0u8; size as usize];
        if GetFileVersionInfoW(
            PCWSTR(path_buf.as_ptr()),
            None,
            size,
            data.as_mut_ptr() as *mut c_void,
        ).is_err() {
            return false;
        }

        // Read the (language, codepage) pair from the translation table, then
        // check the string-table fields Wine populates with its marker.
        let mut translation: *mut c_void = std::ptr::null_mut();
        let mut translation_len = 0u32;
        if !VerQueryValueW(
            data.as_ptr() as *const c_void,
            PCWSTR(w!("\\VarFileInfo\\Translation").as_ptr()),
            &mut translation,
            &mut translation_len,
        ).as_bool()
            || translation_len < 4
            || translation.is_null()
        {
            return false;
        }

        let lang = *(translation as *const u16);
        let codepage = *((translation as *const u16).add(1));

        for field in ["ProductName", "CompanyName", "FileVersion"] {
            let subblock = format!("\\StringFileInfo\\{:04X}{:04X}\\{}", lang, codepage, field);
            let mut subblock_u16: Vec<u16> = subblock.encode_utf16().collect();
            subblock_u16.push(0);

            let mut value: *mut c_void = std::ptr::null_mut();
            let mut value_len = 0u32;
            if VerQueryValueW(
                data.as_ptr() as *const c_void,
                PCWSTR(subblock_u16.as_ptr()),
                &mut value,
                &mut value_len,
            ).as_bool()
                && !value.is_null()
            {
                let chars = std::slice::from_raw_parts(value as *const u16, value_len as usize);
                let end = chars.iter().position(|&c| c == 0).unwrap_or(chars.len());
                if String::from_utf16_lossy(&chars[..end]).to_lowercase().contains("wine") {
                    return true;
                }
            }
        }

        false
    }
}

pub fn on_hooking_finished(hachimi: &Hachimi) {
    wnd_hook::init();

    // Detect Wine/Proton and log it
    if crate::windows::capabilities::is_wine() {
        info!("Wine/Proton detected — disabling SMTC, taskbar progress, and scheduled toast integration");
    }

    // Kill unity crash handler (just to be safe)
    unsafe {
        if let Err(e) = utils::kill_process_by_name(c"UnityCrashHandler64.exe") {
            warn!("Error occured while trying to kill crash handler: {}", e);
        }
    };

    // Apply vsync
    if hachimi.vsync_count.load(atomic::Ordering::Relaxed) != -1 {
        QualitySettings::set_vSyncCount(1);
    }

    // Apply auto full screen
    if hachimi.config.load().windows.auto_full_screen {
        std::thread::spawn(|| {
            std::thread::sleep(std::time::Duration::from_secs(2));
            Thread::main_thread().schedule(|| {
                Screen::apply_auto_full_screen(Screen::get_width(), Screen::get_height());
            });
        });
    }

    // Clean up the update installer
    _ = std::fs::remove_file(utils::get_tmp_installer_path());
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Config {
    #[serde(default = "Config::default_vsync_count")]
    pub vsync_count: i32,
    #[serde(default)]
    pub load_libraries: Vec<String>,
    #[serde(default = "Config::default_menu_open_key")]
    pub menu_open_key: u16,
    #[serde(default = "Config::default_hide_ingame_ui_hotkey_bind")]
    pub hide_ingame_ui_hotkey_bind: u16,
    #[serde(default = "Config::default_race_playback_key")]
    pub race_playback_key: u16,
    #[serde(default)]
    pub auto_full_screen: bool,
    #[serde(default)]
    pub full_screen_mode: FullScreenMode,
    #[serde(default)]
    pub full_screen_res: Resolution,
    #[serde(default)]
    pub resolution_scaling: ResolutionScaling,
    #[serde(default)]
    pub block_minimize_in_full_screen: bool,
    #[serde(default)]
    pub window_always_on_top: bool,
    #[serde(default)]
    pub target_fps_unfocused: Option<i32>,

    #[serde(default = "Config::default_true")]
    pub discord_rpc: bool,
    #[serde(default)]
    pub taskbar_show_progress_on_download: bool,
    #[serde(default)]
    pub taskbar_show_progress_on_connecting: bool,
    #[serde(default = "Config::default_true")]
    pub enable_smtc: bool,
    #[serde(default = "Config::default_true")]
    pub ui_loading_show_orientation_guide: bool,
    #[serde(default = "Config::default_true")]
    pub enable_gui_landscape_ratio: bool,
    #[serde(default = "Config::default_gui_landscape_ratio")]
    pub gui_landscape_ratio: f32,
    #[serde(default, alias = "freeFormWindow")]
    pub freeform_window: bool,
    #[serde(default = "Config::default_true", alias = "freeFormUiScaleAuto")]
    pub freeform_ui_scale_auto: bool,
    #[serde(default = "Config::default_freeform_ui_scale_auto_ratio", alias = "freeFormUiScaleAutoRatio")]
    pub freeform_ui_scale_auto_ratio: f32,
    #[serde(default = "Config::default_one_f32", alias = "freeformUiScalePortrait")]
    pub freeform_ui_scale_portrait: f32,
    #[serde(default = "Config::default_one_f32", alias = "freeformUiScaleLandscape")]
    pub freeform_ui_scale_landscape: f32,
    #[serde(default)]
    pub taskbar_show_progress_on_schedule_book: bool,
    #[serde(default)]
    pub free_camera: super::free_camera::FreeCameraConfig
}

impl Config {
    fn default_vsync_count() -> i32 { -1 }
    fn default_menu_open_key() -> u16 { windows::Win32::UI::Input::KeyboardAndMouse::VK_RIGHT.0 }
    fn default_hide_ingame_ui_hotkey_bind() -> u16 { windows::Win32::UI::Input::KeyboardAndMouse::VK_INSERT.0 }
    fn default_race_playback_key() -> u16 { windows::Win32::UI::Input::KeyboardAndMouse::VK_P.0 }
    fn default_true() -> bool { true }
    fn default_gui_landscape_ratio() -> f32 { 1.0 }
    fn default_freeform_ui_scale_auto_ratio() -> f32 { 0.55 }
    fn default_one_f32() -> f32 { 1.0 }
}

#[derive(Deserialize, Serialize, Copy, Clone, Default, Eq, PartialEq)]
#[repr(i32)]
pub enum FullScreenMode {
    #[default] ExclusiveFullScreen = FullScreenMode_ExclusiveFullScreen,
    FullScreenWindow = FullScreenMode_FullScreenWindow
}

#[derive(Deserialize, Serialize, Copy, Clone, Default, Eq, PartialEq)]
pub enum ResolutionScaling {
    #[default] Default,
    ScaleToScreenSize,
    ScaleToWindowSize
}

impl ResolutionScaling {
    pub fn is_not_default(&self) -> bool { *self != Self::Default }
}
