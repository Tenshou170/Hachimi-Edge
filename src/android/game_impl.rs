use procfs::process::Process;
use std::{ffi::{c_char, c_int, CStr}, path::{Path, PathBuf}, process};

use jni::{
    objects::{JObject, JString},
    sys,
    JNIEnv
};
use once_cell::sync::OnceCell;

use crate::{android::zygisk, core::game::Region};

#[repr(C)]
struct AAssetManager {
    _unused: [u8; 0]
}

#[repr(C)]
struct AAsset {
    _unused: [u8; 0]
}

const AASSET_MODE_UNKNOWN: c_int = 0;

#[link(name = "android")]
extern "C" {
    fn AAssetManager_fromJava(env: *mut sys::JNIEnv, asset_manager: sys::jobject) -> *mut AAssetManager;
    fn AAssetManager_open(mgr: *mut AAssetManager, filename: *const c_char, mode: c_int) -> *mut AAsset;
    fn AAsset_close(asset: *mut AAsset);
}

static INTERNAL_FILES_DIR: OnceCell<Option<PathBuf>> = OnceCell::new();

const INTERNAL_FILES_MARKER: &CStr = c"hachimi_internal_files";

pub fn get_package_name() -> String {
    match zygisk::get_package_name() {
        Some(name) => name.clone(),
        None => {
            let proc = Process::myself().unwrap_or_else(|_| {
                error!("FATAL: Failed to read /proc/self");
                process::exit(1);
            });
            let cmdline = proc.cmdline().unwrap_or_else(|_| {
                error!("FATAL: Failed to read /proc/self/cmdline");
                process::exit(1);
            });
            cmdline.first().unwrap_or_else(|| {
                error!("FATAL: Invalid cmdline");
                process::exit(1);
            }).to_owned()
        }
    }
}

pub fn get_region(package_name: &str) -> Region {
    match package_name {
        "jp.co.cygames.umamusume" => Region::Japan,
        "com.komoe.kmumamusumegp" | "com.komoe.umamusumeofficial" => Region::Taiwan,
        "com.kakaogames.umamusume" => Region::Korea,
        "com.bilibili.umamusu" => Region::China,
        "com.cygames.umamusume" => Region::Global,
        _ => Region::Unknown
    }
}

pub fn get_data_dir(package_name: &str) -> PathBuf {
    // If the APK contains a hachimi_internal_files marker asset (or file), use the
    // app's internal files dir instead of /sdcard/Android/media/<pkg>/hachimi.
    // This enables "Direct Install" APKs that don't require external storage.
    if let Some(dir) = INTERNAL_FILES_DIR.get().and_then(Option::as_ref) {
        return dir.to_path_buf();
    }

    let mut path = Path::new("/sdcard/Android/media").join(package_name);
    path.push("hachimi");
    path
}

/// Call this once during Android JNI init (before get_data_dir is first used).
/// Checks for a `hachimi_internal_files` marker — either as a file in the app's
/// internal files dir or as an asset bundled in the APK — and caches the result.
pub fn check_internal_files_marker(env: &mut JNIEnv) {
    let result = detect_internal_files_dir(env);

    if env.exception_check().unwrap_or(false) {
        warn!("game_impl: JNI exception during hachimi_internal_files marker check");
        let _ = env.exception_describe();
        let _ = env.exception_clear();
    }

    _ = INTERNAL_FILES_DIR.set(result);
}

/// Logs the cached marker result. Call this AFTER log init (e.g. once
/// Hachimi::init has set up logcat) — the check itself runs before the logger
/// exists, so diagnostics emitted there would be silently dropped.
pub fn log_marker_result() {
    match INTERNAL_FILES_DIR.get() {
        Some(Some(_)) => info!("hachimi_internal_files marker found, using the app's internal files dir"),
        Some(None) => info!("hachimi_internal_files marker not found, using /sdcard/Android/media data dir"),
        None => warn!("hachimi_internal_files marker check never ran, using /sdcard/Android/media data dir")
    }
}

fn get_application<'local>(env: &mut JNIEnv<'local>) -> Option<JObject<'local>> {
    let activity_thread_class = env.find_class("android/app/ActivityThread").ok()?;
    let app = env
        .call_static_method(
            activity_thread_class,
            "currentApplication",
            "()Landroid/app/Application;",
            &[]
        )
        .ok()?
        .l()
        .ok()?;

    if app.is_null() {
        return None;
    }
    Some(app)
}

fn detect_internal_files_dir(env: &mut JNIEnv) -> Option<PathBuf> {
    let app = get_application(env)?;

    let files_dir = env
        .call_method(&app, "getFilesDir", "()Ljava/io/File;", &[])
        .ok()?
        .l()
        .ok()?;
    if files_dir.is_null() {
        return None;
    }

    let path = env
        .call_method(&files_dir, "getAbsolutePath", "()Ljava/lang/String;", &[])
        .ok()?
        .l()
        .ok()?;
    if path.is_null() {
        return None;
    }
    let path_str: String = env.get_string(&JString::from(path)).ok()?.into();

    let mut dir = PathBuf::from(path_str);

    // Check for a plain file marker in the internal files dir (simpler deployment).
    let marker_name = INTERNAL_FILES_MARKER.to_str().ok()?;
    if dir.join(marker_name).exists() {
        dir.push("hachimi");
        return Some(dir);
    }

    // Fall back to checking the APK's asset bundle for the marker.
    let assets = env
        .call_method(&app, "getAssets", "()Landroid/content/res/AssetManager;", &[])
        .ok()?
        .l()
        .ok()?;
    if assets.is_null() {
        return None;
    }

    let asset_manager = unsafe { AAssetManager_fromJava(env.get_raw(), assets.as_raw()) };
    if asset_manager.is_null() {
        return None;
    }
    let asset = unsafe {
        AAssetManager_open(asset_manager, INTERNAL_FILES_MARKER.as_ptr(), AASSET_MODE_UNKNOWN)
    };
    if asset.is_null() {
        return None;
    }
    unsafe { AAsset_close(asset) };

    dir.push("hachimi");
    Some(dir)
}
