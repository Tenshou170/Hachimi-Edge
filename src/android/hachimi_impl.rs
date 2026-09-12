use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::core::Hachimi;

use super::gui_impl::keymap;

static KEEP_SCREEN_ON: AtomicBool = AtomicBool::new(false);

pub fn is_il2cpp_lib(filename: &str) -> bool {
    filename.ends_with("libil2cpp.so")
}

pub fn is_criware_lib(filename: &str) -> bool {
    filename.ends_with("libcri_ware_unity.so")
}

pub fn on_hooking_finished(hachimi: &Hachimi) {
    let config = hachimi.config.load();
    set_keep_screen_on(config.android.keep_screen_on);
}

pub fn is_keep_screen_on() -> bool {
    KEEP_SCREEN_ON.load(Ordering::Relaxed)
}

pub fn set_keep_screen_on(enable: bool) {
    // No-op if already in the requested state — avoid redundant IL2CPP calls on
    // every save_and_reload_config (which fires on every Race Director HUD window move).
    if KEEP_SCREEN_ON.load(Ordering::Relaxed) == enable {
        return;
    }
    info!("set_keep_screen_on called (enable={})", enable);
    KEEP_SCREEN_ON.store(enable, Ordering::Relaxed);

    crate::il2cpp::hook::UnityEngine_CoreModule::Screen::set_screen_timeout_disabled(enable);
}

#[allow(dead_code)]
fn set_keep_screen_on_jni(enable: bool) {
    let Some(vm) = crate::android::main::java_vm() else {
        info!("JNI Keep Screen On skipped: Java VM unavailable");
        return;
    };
    let Ok(mut env) = vm.attach_current_thread_as_daemon() else {
        info!("JNI Keep Screen On skipped: failed to attach thread");
        return;
    };

    let result = (|| -> jni::errors::Result<()> {
        let activity = crate::android::utils::get_activity(unsafe { env.unsafe_clone() })
            .ok_or(jni::errors::Error::JavaException)?;

        let window = env.call_method(&activity, "getWindow", "()Landroid/view/Window;", &[])?.l()?;
        if window.is_null() {
            return Err(jni::errors::Error::JavaException);
        }

        let flag_keep_screen_on: i32 = 0x00000080;
        if enable {
            env.call_method(
                &window,
                "addFlags",
                "(I)V",
                &[jni::objects::JValue::Int(flag_keep_screen_on)]
            )?;
            info!("Successfully added FLAG_KEEP_SCREEN_ON to Window");
        } else {
            env.call_method(
                &window,
                "clearFlags",
                "(I)V",
                &[jni::objects::JValue::Int(flag_keep_screen_on)]
            )?;
            info!("Successfully cleared FLAG_KEEP_SCREEN_ON from Window");
        }
        Ok(())
    })();

    if let Err(e) = result {
        info!("JNI Keep Screen On (best-effort) failed: {:?}", e);
        if env.exception_check().unwrap_or(false) {
            let _ = env.exception_clear();
        }
    }
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Config {
    #[serde(default = "Config::default_menu_open_key")]
    pub menu_open_key: i32,
    #[serde(default = "Config::default_hide_ingame_ui_hotkey_bind")]
    pub hide_ingame_ui_hotkey_bind: i32,
    #[serde(default = "Config::default_race_playback_key")]
    pub race_playback_key: i32,
    #[serde(default)]
    pub load_libraries: Vec<String>,
    #[serde(default)]
    pub hook_libc_dlopen: bool,
    #[serde(default)]
    pub keep_screen_on: bool,
    #[serde(default)]
    pub enable_gui_landscape_ratio: bool,
    #[serde(default = "Config::default_gui_landscape_ratio")]
    pub gui_landscape_ratio: f32,
    #[serde(default)]
    pub force_orientation_mode: i32,
}

impl Config {
    fn default_menu_open_key() -> i32 { keymap::KEYCODE_DPAD_RIGHT }
    fn default_hide_ingame_ui_hotkey_bind() -> i32 { keymap::KEYCODE_INSERT }
    fn default_race_playback_key() -> i32 { keymap::KEYCODE_P }
    fn default_gui_landscape_ratio() -> f32 { 1.0 }
}
