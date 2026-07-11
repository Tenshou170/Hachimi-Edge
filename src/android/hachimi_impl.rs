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

pub fn on_hooking_finished(_hachimi: &Hachimi) {
}

pub fn is_keep_screen_on() -> bool {
    KEEP_SCREEN_ON.load(Ordering::Relaxed)
}

/// Update the keep-screen-on state.
///
/// The primary mechanism is the IL2CPP `Screen.sleepTimeout` hook which
/// intercepts Unity's own timeout setter and is always called from the
/// correct thread.  The JNI `Window.addFlags(FLAG_KEEP_SCREEN_ON)` path
/// is attempted as a best-effort supplement — it may fail silently when
/// called from a non-UI thread, which is expected.
pub fn set_keep_screen_on(enable: bool) {
    info!("set_keep_screen_on called (enable={})", enable);
    KEEP_SCREEN_ON.store(enable, Ordering::Relaxed);

    // Primary: set Unity's sleepTimeout (thread-safe, works from any thread)
    crate::il2cpp::hook::UnityEngine_CoreModule::Screen::set_screen_timeout_disabled(enable);

    // Best-effort: set Android Window flag (may fail if not on UI thread)
    set_keep_screen_on_jni(enable);
}

/// Best-effort attempt to add/clear `FLAG_KEEP_SCREEN_ON` on the Activity
/// Window via JNI.  This can fail when called from a non-UI thread; the
/// IL2CPP sleepTimeout hook is the reliable fallback.
fn set_keep_screen_on_jni(enable: bool) {
    let Some(vm) = crate::android::main::java_vm() else {
        info!("JNI Keep Screen On skipped: Java VM unavailable");
        return;
    };
    let Ok(mut env) = vm.attach_current_thread() else {
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
    #[serde(default)]
    pub load_libraries: Vec<String>,
    #[serde(default)]
    pub hook_libc_dlopen: bool,
    #[serde(default)]
    pub keep_screen_on: bool
}

impl Config {
    fn default_menu_open_key() -> i32 { keymap::KEYCODE_DPAD_RIGHT }
    fn default_hide_ingame_ui_hotkey_bind() -> i32 { keymap::KEYCODE_INSERT }
}
