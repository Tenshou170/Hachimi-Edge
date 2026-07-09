use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::core::Hachimi;

use super::gui_impl::keymap;

static KEEP_SCREEN_ON: AtomicBool = AtomicBool::new(false);
static KEEP_SCREEN_ON_PENDING: AtomicBool = AtomicBool::new(false);
static KEEP_SCREEN_ON_RENDER_READY: AtomicBool = AtomicBool::new(false);

pub fn is_il2cpp_lib(filename: &str) -> bool {
    filename.ends_with("libil2cpp.so")
}

pub fn is_criware_lib(filename: &str) -> bool {
    filename.ends_with("libcri_ware_unity.so")
}

pub fn on_hooking_finished(_hachimi: &Hachimi) {
}

pub fn set_keep_screen_on(enable: bool) {
    info!("set_keep_screen_on called (enable={})", enable);
    KEEP_SCREEN_ON.store(enable, Ordering::Relaxed);
    KEEP_SCREEN_ON_PENDING.store(true, Ordering::Relaxed);

    if KEEP_SCREEN_ON_RENDER_READY.load(Ordering::Relaxed) {
        set_keep_screen_on_jni(enable);
        KEEP_SCREEN_ON_PENDING.store(false, Ordering::Relaxed);
    } else {
        info!("keep_screen_on update deferred until render hook is ready");
    }
}

pub fn apply_keep_screen_on_if_pending() {
    KEEP_SCREEN_ON_RENDER_READY.store(true, Ordering::Relaxed);
    let enable = KEEP_SCREEN_ON.load(Ordering::Relaxed);
    if KEEP_SCREEN_ON_PENDING.swap(false, Ordering::Relaxed) || enable {
        set_keep_screen_on_jni(enable);
    }
}

fn set_keep_screen_on_jni(enable: bool) {
    let Some(vm) = crate::android::main::java_vm() else {
        return;
    };
    let Ok(mut env) = vm.attach_current_thread() else {
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
        info!("JNI Keep Screen On Error: {:?}", e);
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
