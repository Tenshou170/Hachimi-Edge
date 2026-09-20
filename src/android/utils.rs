use jni::{
    objects::{JValue, JMap, JObject, JString},
    JNIEnv
};
use crate::{
    android::main::java_vm,
    il2cpp::{ext::StringExt, hook::UnityEngine_CoreModule::Application}
};

use std::{path::PathBuf, sync::atomic::{AtomicBool, Ordering}};
use super::game_impl;

pub static BACK_BUTTON_PRESSED: AtomicBool = AtomicBool::new(false);
pub static IS_IME_VISIBLE: AtomicBool = AtomicBool::new(false);

pub fn set_keyboard_visible(visible: bool) {
    let Some(vm) = java_vm() else {
        return;
    };
    let Ok(mut env) = vm.attach_current_thread_as_daemon() else {
        return;
    };

    let api_level = crate::android::hook::cached_api_level();

    let result = (|| -> jni::errors::Result<()> {
        let activity = get_activity(unsafe { env.unsafe_clone() })
            .ok_or(jni::errors::Error::JavaException)?;

        let window = env.call_method(&activity, "getWindow", "()Landroid/view/Window;", &[])?.l()?;
        if window.is_null() {
            return Err(jni::errors::Error::JavaException);
        }
        let decor_view = env.call_method(&window, "getDecorView", "()Landroid/view/View;", &[])?.l()?;
        if decor_view.is_null() {
            return Err(jni::errors::Error::JavaException);
        }

        if api_level >= 30 {
            let controller = env.call_method(
                &window,
                "getInsetsController",
                "()Landroid/view/WindowInsetsController;",
                &[],
            )?.l()?;
            if controller.is_null() {
                return Err(jni::errors::Error::JavaException);
            }
            let ime_type: i32 = 8;
            if visible {
                env.call_method(&controller, "show", "(I)V", &[JValue::Int(ime_type)])?;
            } else {
                env.call_method(&controller, "hide", "(I)V", &[JValue::Int(ime_type)])?;
            }
        } else {
            let context_class = env.find_class("android/content/Context")?;
            let imm_service_name = env.get_static_field(context_class, "INPUT_METHOD_SERVICE", "Ljava/lang/String;")?.l()?;
            if imm_service_name.is_null() {
                return Err(jni::errors::Error::JavaException);
            }
            let imm = env.call_method(
                &activity,
                "getSystemService",
                "(Ljava/lang/String;)Ljava/lang/Object;",
                &[JValue::from(&imm_service_name)],
            )?.l()?;
            if imm.is_null() {
                return Err(jni::errors::Error::JavaException);
            }
            if visible {
                let focus_view = env.call_method(&window, "getCurrentFocus", "()Landroid/view/View;", &[])?.l()?;
                let target_view = if !focus_view.is_null() { &focus_view } else { &decor_view };
                let shown = env.call_method(
                    &imm,
                    "showSoftInput",
                    "(Landroid/view/View;I)Z",
                    &[JValue::from(target_view), JValue::Int(1)],
                )?.z()?;
                if !shown {
                    env.call_method(&imm, "showSoftInput", "(Landroid/view/View;I)Z",
                        &[JValue::from(target_view), JValue::Int(2)])?;
                }
            } else {
                let window_token = env.call_method(&decor_view, "getWindowToken", "()Landroid/os/IBinder;", &[])?.l()?;
                if !window_token.is_null() {
                    env.call_method(
                        &imm,
                        "hideSoftInputFromWindow",
                        "(Landroid/os/IBinder;I)Z",
                        &[JValue::from(&window_token), JValue::Int(0)],
                    )?;
                }
            }
        }
        IS_IME_VISIBLE.store(visible, Ordering::Release);
        Ok(())
    })();

    if let Err(e) = result {
        info!("JNI Keyboard Error: {:?}", e);
        if env.exception_check().unwrap_or(false) {
            let _ = env.exception_clear();
        }
    }
}

pub fn check_keyboard_status() -> bool {
    let vm = match java_vm() {
        Some(v) => v,
        None => return false,
    };
    let mut env = match vm.attach_current_thread_as_daemon() {
        Ok(e) => e,
        Err(_) => return false,
    };
    let api_level = crate::android::hook::cached_api_level();

    let is_visible = (|| -> jni::errors::Result<bool> {
        let activity = get_activity(unsafe { env.unsafe_clone() }).ok_or(jni::errors::Error::JavaException)?;
        let window = env.call_method(&activity, "getWindow", "()Landroid/view/Window;", &[])?.l()?;
        let decor_view = env.call_method(&window, "getDecorView", "()Landroid/view/View;", &[])?.l()?;

        if api_level >= 30 {
            let root_insets = env.call_method(&decor_view, "getRootWindowInsets", "()Landroid/view/WindowInsets;", &[])?.l()?;
            if !root_insets.is_null() {
                let ime_type = 8;
                return env.call_method(&root_insets, "isVisible", "(I)Z", &[JValue::Int(ime_type)])?.z();
            }
            return Ok(false);
        }

        let rect_class = env.find_class("android/graphics/Rect")?;
        let rect_obj = env.new_object(&rect_class, "()V", &[])?;
        env.call_method(&decor_view, "getWindowVisibleDisplayFrame", "(Landroid/graphics/Rect;)V", &[JValue::from(&rect_obj)])?;
        let display_height = env.call_method(&decor_view, "getHeight", "()I", &[])?.i()?;
        let visible_bottom = env.get_field(&rect_obj, "bottom", "I")?.i()?;

        let height_diff = display_height - visible_bottom;
        Ok(height_diff > (display_height / 4))
    })();

    let is_visible = match is_visible {
        Ok(v) => v,
        Err(_) => {
            if env.exception_check().unwrap_or(false) {
                let _ = env.exception_clear();
            }
            false
        }
    };

    let old = IS_IME_VISIBLE.swap(is_visible, Ordering::AcqRel);
    if old != is_visible {
        info!("Keyboard visibility changed: {}", is_visible);
    }
    is_visible
}

#[allow(dead_code)]
pub fn open_app_or_fallback(package_name: &str, activity_class: &str, fallback_url: &str) {
    let vm = match java_vm() {
        Some(v) => v,
        None => return,
    };

    let mut env = match vm.attach_current_thread_as_daemon() {
        Ok(e) => e,
        Err(_) => return,
    };

    let try_open = |env: &mut JNIEnv| -> jni::errors::Result<()> {
        let activity = get_activity(unsafe { env.unsafe_clone() }).ok_or(jni::errors::Error::JavaException)?;

        let intent_class = env.find_class("android/content/Intent")?;

        let intent_obj = env.new_object(&intent_class, "()V", &[])?;

        let pkg_name_java = env.new_string(package_name)?;
        let cls_name_java = env.new_string(activity_class)?;
        
        let component_class = env.find_class("android/content/ComponentName")?;
        let component_obj = env.new_object(
            &component_class, 
            "(Ljava/lang/String;Ljava/lang/String;)V", 
            &[JValue::from(&pkg_name_java), JValue::from(&cls_name_java)]
        )?;
    
        env.call_method(
            &intent_obj, 
            "setComponent", 
            "(Landroid/content/ComponentName;)Landroid/content/Intent;", 
            &[JValue::from(&component_obj)]
        )?;

        env.call_method(&intent_obj, "setFlags", "(I)Landroid/content/Intent;", &[JValue::Int(0x10000000)])?;

        env.call_method(&activity, "startActivity", "(Landroid/content/Intent;)V", &[JValue::from(&intent_obj)])?;
        Ok(())
    };

    if let Err(_e) = try_open(&mut env) {
        if env.exception_check().unwrap_or(false) {
            if let Ok(ex) = env.exception_occurred() {
                let _ = env.exception_clear();

                if let Ok(msg_obj) = env.call_method(ex, "toString", "()Ljava/lang/String;", &[]) {
                    if let Ok(msg_jstr_obj) = msg_obj.l() {
                        let msg_jstr: JString = msg_jstr_obj.into();
                        let msg_rust = env.get_string(&msg_jstr);
                        if let Ok(msg_rust) = msg_rust {
                            let msg_str: String = msg_rust.into();
                            info!("open_app_or_fallback: Java Exception: {}", msg_str);
                        }
                    }
                }
            }
        }
        
        info!("open_app_or_fallback: Launch failed for {}, falling back to URL {}", package_name, fallback_url);
        let url_ptr = fallback_url.to_il2cpp_string();
        Application::OpenURL(url_ptr);
    }
}

pub fn get_activity(mut env: JNIEnv<'_>) -> Option<JObject<'_>> {
    // 1. Try cached UnityPlayer class first (cached in JNI_OnLoad with app class loader)
    if let Some(unity_player_gref) = super::main::get_unity_player_class() {
        let unity_player_class: &jni::objects::JClass = unity_player_gref.as_obj().into();
        if let Ok(current_activity_val) = env.get_static_field(
            unity_player_class,
            "currentActivity",
            "Landroid/app/Activity;",
        ) {
            if let Ok(current_activity) = current_activity_val.l() {
                if !current_activity.is_null() {
                    debug!("get_activity: Found via cached UnityPlayer.currentActivity");
                    return Some(current_activity);
                }
            }
        }
        if env.exception_check().unwrap_or(false) {
            let _ = env.exception_clear();
        }
    }

    // 2. Fallback: find UnityPlayer dynamically (works if Java frames are on stack)
    if let Ok(unity_player_class) = env.find_class("com/unity3d/player/UnityPlayer") {
        if let Ok(current_activity_val) = env.get_static_field(
            unity_player_class,
            "currentActivity",
            "Landroid/app/Activity;",
        ) {
            if let Ok(current_activity) = current_activity_val.l() {
                if !current_activity.is_null() {
                    debug!("get_activity: Found via dynamic UnityPlayer.currentActivity");
                    return Some(current_activity);
                }
            }
        }
    }
    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }

    // 3. Fallback: ActivityThread.mActivities (best effort for non-standard hosts)
    debug!("get_activity: Trying ActivityThread fallback");
    if let Ok(at_class) = env.find_class("android/app/ActivityThread") {
        if env.exception_check().unwrap_or(false) { let _ = env.exception_clear(); }
        if let Ok(at_val) = env.call_static_method(
            &at_class, "currentActivityThread", "()Landroid/app/ActivityThread;", &[]
        ) {
            if let Ok(at) = at_val.l() {
                if !at.is_null() {
                    debug!("get_activity: Got ActivityThread instance");
                    if let Ok(activities_val) = env.get_field(&at, "mActivities", "Landroid/util/ArrayMap;") {
                        if let Ok(activities) = activities_val.l() {
                            if !activities.is_null() {
                                debug!("get_activity: Iterating mActivities map");
                                if let Ok(activities_map) = JMap::from_env(&mut env, &activities) {
                                    if let Ok(mut iter) = activities_map.iter(&mut env) {
                                        while let Ok(Some((_, record))) = iter.next(&mut env) {
                                            if let Ok(av) = env.get_field(&record, "activity", "Landroid/app/Activity;") {
                                                if let Ok(act) = av.l() {
                                                    if !act.is_null() {
                                                        info!("get_activity: Found activity in mActivities");
                                                        return Some(act);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if env.exception_check().unwrap_or(false) { let _ = env.exception_clear(); }
                }
            }
        }
    }

    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }
    
    warn!("get_activity: Failed to retrieve Activity from any source");
    None
}

pub fn get_context<'a>(env: &mut JNIEnv<'a>) -> Option<JObject<'a>> {
    if let Some(activity) = get_activity(unsafe { env.unsafe_clone() }) {
        return Some(activity);
    }
    if let Ok(at_class) = env.find_class("android/app/ActivityThread") {
        if let Ok(app_val) = env.call_static_method(
            &at_class, "currentApplication", "()Landroid/app/Application;", &[]
        ) {
            if let Ok(app) = app_val.l() {
                if !app.is_null() {
                    debug!("get_context: Found Application via ActivityThread.currentApplication()");
                    return Some(app);
                }
            }
        }
    }
    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }
    None
}

pub fn get_device_api_level(env: *mut jni::sys::JNIEnv) -> i32 {
    let Ok(mut env) = (unsafe { JNIEnv::from_raw(env) }) else {
        return 0;
    };
    env.get_static_field("android/os/Build$VERSION", "SDK_INT", "I")
        .ok()
        .and_then(|value| value.i().ok())
        .unwrap_or(0)
}

pub fn get_safe_insets_jni() -> (f32, f32) {
    let Some(vm) = java_vm() else { return (0.0, 0.0); };
    let Ok(mut env) = vm.attach_current_thread_as_daemon() else { return (0.0, 0.0); };

    let is_ok = (|| -> jni::errors::Result<(f32, f32)> {
        let activity = get_activity(unsafe { env.unsafe_clone() }).ok_or(jni::errors::Error::JavaException)?;
        let window = env.call_method(&activity, "getWindow", "()Landroid/view/Window;", &[])?.l()?;
        if window.is_null() { return Err(jni::errors::Error::JavaException); }
        let decor_view = env.call_method(&window, "getDecorView", "()Landroid/view/View;", &[])?.l()?;
        if decor_view.is_null() { return Err(jni::errors::Error::JavaException); }
        let insets = env.call_method(&decor_view, "getRootWindowInsets", "()Landroid/view/WindowInsets;", &[])?.l()?;
        if insets.is_null() { return Err(jni::errors::Error::JavaException); }

        let api_level = crate::android::hook::cached_api_level();
        let mut top = 0.0f32;
        let mut bottom = 0.0f32;

        if api_level >= 28 {
            if let Ok(cutout) = env.call_method(&insets, "getDisplayCutout", "()Landroid/view/DisplayCutout;", &[]) {
                if let Ok(cutout) = cutout.l() {
                    if !cutout.is_null() {
                        if let Ok(t) = env.call_method(&cutout, "getSafeInsetTop", "()I", &[]).and_then(|v| v.i()) {
                            top = top.max(t as f32);
                        }
                        if let Ok(b) = env.call_method(&cutout, "getSafeInsetBottom", "()I", &[]).and_then(|v| v.i()) {
                            bottom = bottom.max(b as f32);
                        }
                    }
                }
            }
        }

        // If bottom inset is 0 from cutout, check navigation / system bars
        if bottom == 0.0 {
            if api_level >= 30 {
                // Type.systemBars() = 7 (statusBars = 1 | navigationBars = 2 | captionBar = 4)
                if let Ok(insets_obj) = env.call_method(&insets, "getInsets", "(I)Landroid/graphics/Insets;", &[jni::objects::JValue::Int(7)]) {
                    if let Ok(insets_obj) = insets_obj.l() {
                        if !insets_obj.is_null() {
                            if let Ok(b) = env.get_field(&insets_obj, "bottom", "I").and_then(|v| v.i()) {
                                bottom = bottom.max(b as f32);
                            }
                            if top == 0.0 {
                                if let Ok(t) = env.get_field(&insets_obj, "top", "I").and_then(|v| v.i()) {
                                    top = top.max(t as f32);
                                }
                            }
                        }
                    }
                }
            } else {
                if let Ok(b) = env.call_method(&insets, "getSystemWindowInsetBottom", "()I", &[]) {
                    if let Ok(b) = b.i() {
                        bottom = bottom.max(b as f32);
                    }
                }
                if top == 0.0 {
                    if let Ok(t) = env.call_method(&insets, "getSystemWindowInsetTop", "()I", &[]) {
                        if let Ok(t) = t.i() {
                            top = top.max(t as f32);
                        }
                    }
                }
            }
        }

        Ok((top, bottom))
    })();

    match is_ok {
        Ok(insets) => insets,
        Err(_) => {
            if env.exception_check().unwrap_or(false) {
                let _ = env.exception_clear();
            }
            (0.0, 0.0)
        }
    }
}

pub fn get_screen_dimensions(mut env: JNIEnv) -> (i32, i32) {
    let Some(activity) = get_activity(unsafe { env.unsafe_clone() }) else { 
        warn!("get_screen_dimensions: Activity not available, returning default (0, 0)");
        return (0, 0) 
    };

    let result = (|| -> jni::errors::Result<(i32, i32)> {
        let api_level = crate::android::hook::cached_api_level();
        info!("get_screen_dimensions: API level {}", api_level);

        if api_level >= 30 {
            let wm = env.call_method(&activity, "getWindowManager", "()Landroid/view/WindowManager;", &[])?.l()?;
            if wm.is_null() {
                warn!("get_screen_dimensions: WindowManager is null (API 30+)");
                return Err(jni::errors::Error::JavaException);
            }
            let metrics = env.call_method(&wm, "getCurrentWindowMetrics", "()Landroid/view/WindowMetrics;", &[])?.l()?;
            if metrics.is_null() {
                warn!("get_screen_dimensions: WindowMetrics is null");
                return Err(jni::errors::Error::JavaException);
            }
            let bounds = env.call_method(&metrics, "getBounds", "()Landroid/graphics/Rect;", &[])?.l()?;
            if bounds.is_null() {
                warn!("get_screen_dimensions: Rect bounds is null");
                return Err(jni::errors::Error::JavaException);
            }
            let width  = env.get_field(&bounds, "right",  "I")?.i()?;
            let height = env.get_field(&bounds, "bottom", "I")?.i()?;
            info!("get_screen_dimensions (API 30+): {}x{}", width, height);
            Ok((width, height))
        } else {
            let wm      = env.call_method(&activity, "getWindowManager", "()Landroid/view/WindowManager;", &[])?.l()?;
            if wm.is_null() {
                warn!("get_screen_dimensions: WindowManager is null (API <30)");
                return Err(jni::errors::Error::JavaException);
            }
            let display = env.call_method(&wm, "getDefaultDisplay", "()Landroid/view/Display;", &[])?.l()?;
            if display.is_null() {
                warn!("get_screen_dimensions: Display is null");
                return Err(jni::errors::Error::JavaException);
            }
            let dm_class = env.find_class("android/util/DisplayMetrics")?;
            let dm       = env.new_object(dm_class, "()V", &[])?;
            env.call_method(&display, "getRealMetrics", "(Landroid/util/DisplayMetrics;)V", &[JValue::from(&dm)])?;
            let width  = env.get_field(&dm, "widthPixels",  "I")?.i()?;
            let height = env.get_field(&dm, "heightPixels", "I")?.i()?;
            info!("get_screen_dimensions (API <30): {}x{}", width, height);
            Ok((width, height))
        }
    })();

    match result {
        Ok(dims) => {
            if dims.0 == 0 || dims.1 == 0 {
                error!("get_screen_dimensions: Invalid dimensions received: {}x{}", dims.0, dims.1);
                (0, 0)
            } else {
                dims
            }
        }
        Err(e) => {
            error!("get_screen_dimensions: Failed to retrieve dimensions: {:?}", e);
            if env.exception_check().unwrap_or(false) {
                let _ = env.exception_clear();
            }
            (0, 0)
        }
    }
}

pub fn set_audio_capture_policy_all() {
    std::thread::Builder::new()
        .name("audio_capture_policy".into())
        .spawn(|| {
            let Some(vm) = java_vm() else {
                return;
            };
            let Ok(mut env) = vm.attach_current_thread_as_daemon() else {
                return;
            };

            let api_level = crate::android::hook::cached_api_level();
            if api_level < 29 {
                info!("setAllowedCapturePolicy ignored: API level {} is below 29", api_level);
                return;
            }

            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            loop {
                if let Some(ctx) = get_context(&mut env) {
                    let result = (|| -> jni::errors::Result<()> {
                        let context_class = env.find_class("android/content/Context")?;
                        let audio_service_str = env.get_static_field(context_class, "AUDIO_SERVICE", "Ljava/lang/String;")?.l()?;

                        let audio_manager = env.call_method(
                            &ctx, 
                            "getSystemService", 
                            "(Ljava/lang/String;)Ljava/lang/Object;", 
                            &[JValue::from(&audio_service_str)]
                        )?.l()?;

                        if audio_manager.is_null() {
                            return Err(jni::errors::Error::JavaException);
                        }

                        let allow_capture_by_all: i32 = 1;
                        env.call_method(
                            &audio_manager, 
                            "setAllowedCapturePolicy", 
                            "(I)V", 
                            &[JValue::Int(allow_capture_by_all)]
                        )?;

                        info!("Successfully set AudioManager capture policy to ALLOW_CAPTURE_BY_ALL");
                        Ok(())
                    })();

                    match result {
                        Ok(()) => return,
                        Err(e) => {
                            debug!("set_audio_capture_policy_all attempt error: {:?}", e);
                            if env.exception_check().unwrap_or(false) {
                                let _ = env.exception_clear();
                            }
                        }
                    }
                }

                if std::time::Instant::now() >= deadline {
                    warn!("set_audio_capture_policy_all: Timed out waiting for Android Context");
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
        })
        .ok();
}

pub fn get_game_dir() -> PathBuf {
    let package_name = game_impl::get_package_name();
    game_impl::get_data_dir(&package_name)
}
