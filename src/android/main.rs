use std::{ffi::CStr, os::raw::c_void};
use jni::{objects::GlobalRef, sys::jint, JavaVM};
use once_cell::sync::OnceCell;

use crate::core::Hachimi;

use super::{game_impl, hook, plugin_loader};

#[allow(non_camel_case_types)]
type JniOnLoadFn = extern "C" fn(vm: JavaVM, reserved: *mut c_void) -> jint;

const LIBRARY_NAME: &CStr = c"libmain_orig.so";
const JNI_ONLOAD_NAME: &CStr = c"JNI_OnLoad";

static JAVA_VM: OnceCell<JavaVM> = OnceCell::new();
static UNITY_PLAYER_CLASS: OnceCell<GlobalRef> = OnceCell::new();

pub(crate) fn java_vm() -> Option<&'static JavaVM> {
    JAVA_VM.get()
}

pub(crate) fn get_unity_player_class() -> Option<&'static GlobalRef> {
    UNITY_PLAYER_CLASS.get()
}

fn resolve_orig_jni_onload() -> Option<JniOnLoadFn> {
    unsafe {
        let handle = libc::dlopen(LIBRARY_NAME.as_ptr(), libc::RTLD_LAZY);
        if handle.is_null() {
            let err = libc::dlerror();
            let err_str = if err.is_null() {
                "(no dlerror)".into()
            } else {
                std::ffi::CStr::from_ptr(err).to_string_lossy()
            };
            error!(
                "JNI_OnLoad: dlopen({}) RTLD_LAZY failed: {}",
                LIBRARY_NAME.to_string_lossy(),
                err_str
            );
            return None;
        }
        let sym = libc::dlsym(handle, JNI_ONLOAD_NAME.as_ptr());
        if sym.is_null() {
            error!(
                "JNI_OnLoad: JNI_OnLoad symbol not found in {}",
                LIBRARY_NAME.to_string_lossy()
            );
            return None;
        }
        Some(std::mem::transmute(sym))
    }
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn JNI_OnLoad(vm: JavaVM, reserved: *mut c_void) -> jint {
    let Some(orig_fn) = resolve_orig_jni_onload() else {
        error!("JNI_OnLoad: falling back without the original JNI entrypoint");
        return jni::sys::JNI_VERSION_1_6;
    };

    let vm_ptr = vm.get_java_vm_pointer();

    // Check for the hachimi_internal_files marker BEFORE Hachimi::init():
    // init resolves game.data_dir through game_impl::get_data_dir, which depends
    // on the marker result to select the internal files dir (upstream 7b2e994).
    match unsafe { JavaVM::from_raw(vm_ptr) } {
        Ok(vm_for_env) => match vm_for_env.get_env() {
            Ok(mut env) => {
                game_impl::check_internal_files_marker(&mut env);
            }
            Err(e) => {
                error!("JNI_OnLoad: Failed to get JNI env for marker check: {:?}", e);
            }
        },
        Err(e) => {
            error!("JNI_OnLoad: Failed to reconstruct JavaVM for marker check: {:?}", e);
        }
    }

    if !Hachimi::init() {
        return orig_fn(vm, reserved);
    }
    
    if let Err(_) = JAVA_VM.set(vm) {
        error!("JAVA_VM already initialized");
        // Create new wrapper for orig_fn call
        let vm_for_orig = unsafe { JavaVM::from_raw(vm_ptr).expect("Failed to reconstruct JavaVM") };
        return orig_fn(vm_for_orig, reserved);
    }
    
    let hachimi = Hachimi::instance();
    game_impl::log_marker_result();
    
    match hachimi.plugins.lock() {
        Ok(mut plugins) => {
            *plugins = plugin_loader::load_libraries();
        }
        Err(e) => {
            error!("Failed to acquire plugins lock: {:?}", e);
            let vm_for_orig = unsafe { JavaVM::from_raw(vm_ptr).expect("Failed to reconstruct JavaVM") };
            return orig_fn(vm_for_orig, reserved);
        }
    }
    
    match JAVA_VM.get() {
        Some(stored_vm) => {
            match stored_vm.get_env() {
                Ok(mut env) => {
                    if let Ok(local_class) = env.find_class("com/unity3d/player/UnityPlayer") {
                        if let Ok(global_ref) = env.new_global_ref(local_class) {
                            let _ = UNITY_PLAYER_CLASS.set(global_ref);
                            info!("JNI_OnLoad: Cached UnityPlayer class reference");
                        }
                    }
                    if env.exception_check().unwrap_or(false) {
                        let _ = env.exception_clear();
                    }
                    hook::init(env.get_raw());
                    info!("JNI_OnLoad: Hooks initialized successfully");
                }
                Err(e) => {
                    error!("Failed to get JNI environment: {:?}", e);
                    let vm_for_orig = unsafe { JavaVM::from_raw(vm_ptr).expect("Failed to reconstruct JavaVM") };
                    return orig_fn(vm_for_orig, reserved);
                }
            }
        }
        None => {
            error!("Failed to retrieve stored JavaVM reference");
            let vm_for_orig = unsafe { JavaVM::from_raw(vm_ptr).expect("Failed to reconstruct JavaVM") };
            return orig_fn(vm_for_orig, reserved);
        }
    }

    info!("JNI_OnLoad: Initialization completed successfully");
    let vm_for_orig = unsafe { JavaVM::from_raw(vm_ptr).expect("Failed to reconstruct JavaVM") };
    orig_fn(vm_for_orig, reserved)
}
