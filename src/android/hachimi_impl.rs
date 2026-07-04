use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::core::Hachimi;

use super::gui_impl::keymap;

static KEEP_SCREEN_ON: AtomicBool = AtomicBool::new(false);
static KEEP_SCREEN_ON_PENDING: AtomicBool = AtomicBool::new(false);

pub fn is_il2cpp_lib(filename: &str) -> bool {
    filename.ends_with("libil2cpp.so")
}

pub fn is_criware_lib(filename: &str) -> bool {
    filename.ends_with("libcri_ware_unity.so")
}

pub fn on_hooking_finished(hachimi: &Hachimi) {
    // When hooking finishes we may already have a config request for
    // keep_screen_on even if the GUI is disabled. Schedule apply now and
    // retry until the Activity window becomes available.
    set_keep_screen_on(hachimi.config.load().android.keep_screen_on);
}

/// Sets or clears FLAG_KEEP_SCREEN_ON (0x80) on the game's window.
/// The flag must be applied on the Android UI/main thread — calling
/// addFlags/clearFlags from a render or background thread raises
/// CalledFromWrongThreadException. We schedule through the IL2CPP main
/// thread (which Unity runs on the Android UI thread) to guarantee this.
pub fn set_keep_screen_on(enable: bool) {
    KEEP_SCREEN_ON.store(enable, Ordering::Relaxed);
    KEEP_SCREEN_ON_PENDING.store(true, Ordering::Relaxed);
    crate::il2cpp::symbols::Thread::main_thread().schedule(apply_keep_screen_on_task);
}

fn apply_keep_screen_on_task() {
    if !KEEP_SCREEN_ON_PENDING.load(Ordering::Relaxed) {
        return;
    }

    let enable = KEEP_SCREEN_ON.load(Ordering::Relaxed);
    match apply_keep_screen_on(enable) {
        Ok(()) => {
            KEEP_SCREEN_ON_PENDING.store(false, Ordering::Relaxed);
        }
        Err(ApplyError::ActivityUnavailable) => {
            crate::il2cpp::symbols::Thread::main_thread().schedule(apply_keep_screen_on_task);
        }
        Err(err) => {
            KEEP_SCREEN_ON_PENDING.store(false, Ordering::Relaxed);
            warn!("set_keep_screen_on({}): {:?}", enable, err);
        }
    }
}

#[derive(Debug)]
enum ApplyError {
    ActivityUnavailable,
    JniError,
}

impl From<jni::errors::Error> for ApplyError {
    fn from(_: jni::errors::Error) -> Self {
        ApplyError::JniError
    }
}

fn apply_keep_screen_on(enable: bool) -> Result<(), ApplyError> {
    let Some(vm) = super::main::java_vm() else { return Err(ApplyError::ActivityUnavailable); };
    let Ok(mut env) = vm.attach_current_thread() else { return Err(ApplyError::ActivityUnavailable); };

    let activity = match super::utils::get_activity(unsafe { env.unsafe_clone() }) {
        Some(activity) => activity,
        None => return Err(ApplyError::ActivityUnavailable),
    };

    let result = (|| -> jni::errors::Result<()> {
        let window = env.call_method(&activity, "getWindow", "()Landroid/view/Window;", &[])?.l()?;
        if window.is_null() {
            return Err(jni::errors::Error::JavaException);
        }
        const FLAG_KEEP_SCREEN_ON: i32 = 0x00000080;
        if enable {
            env.call_method(window, "addFlags", "(I)V", &[jni::objects::JValue::Int(FLAG_KEEP_SCREEN_ON)])?;
        } else {
            env.call_method(window, "clearFlags", "(I)V", &[jni::objects::JValue::Int(FLAG_KEEP_SCREEN_ON)])?;
        }
        Ok(())
    })();

    if let Err(_e) = result {
        if env.exception_check().unwrap_or(false) {
            let _ = env.exception_clear();
            return Err(ApplyError::ActivityUnavailable);
        }
        return Err(ApplyError::JniError);
    }

    Ok(())
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
