use std::os::raw::c_long;

use jni::{objects::JString, JNIEnv};

use crate::{android::{game_impl, hook, plugin_loader, zygisk::{internal::{api_table, module_abi}, AppSpecializeArgs, ServerSpecializeArgs}}, core::{game::Region, Hachimi}};

const ZYGISK_API_VERSION: c_long = 4;

pub struct Module {
    env: *mut jni::sys::JNIEnv,
    is_game: bool
}

unsafe impl Send for Module {}
unsafe impl Sync for Module {}

impl Module {
    fn new(env: *mut jni::sys::JNIEnv) -> Self {
        Self {
            env,
            is_game: false
        }
    }
}

use once_cell::sync::OnceCell as SyncOnceCell;

static PACKAGE_NAME: SyncOnceCell<String> = SyncOnceCell::new();
pub fn get_package_name() -> Option<&'static String> {
    PACKAGE_NAME.get()
}

unsafe extern "C" fn pre_app_specialize(this: *mut Module, args: *mut AppSpecializeArgs) {
    let mut env = unsafe { JNIEnv::from_raw((*this).env).unwrap() };
    let jstr = JString::from_raw(*(*args).nice_name);
    let java_str = env.get_string(&jstr).unwrap();
    let package_name = java_str.to_string_lossy();
    _ = PACKAGE_NAME.set(package_name.to_string());

    (*this).is_game = game_impl::get_region(&package_name) != Region::Unknown;
}

unsafe extern "C" fn post_app_specialize(this: *mut Module, _args: *const AppSpecializeArgs) {
    if (*this).is_game {
        // Check the hachimi_internal_files marker BEFORE Hachimi::init(): init
        // resolves game.data_dir through game_impl::get_data_dir, which depends
        // on the marker result to select the internal files dir.
        let mut env = unsafe { JNIEnv::from_raw((*this).env).unwrap() };
        game_impl::check_internal_files_marker(&mut env);

        if !Hachimi::init() {
            return;
        }
        let hachimi = Hachimi::instance();
        game_impl::log_marker_result();
        let _ = hachimi.plugins.lock().map(|mut plugins| {
            *plugins = plugin_loader::load_libraries();
        });
        hook::init((*this).env);
    }
}

unsafe extern "C" fn pre_server_specialize(_this: *mut Module, _args: *mut ServerSpecializeArgs) {

}

unsafe extern "C" fn post_server_specialize(_this: *mut Module, _args: *const ServerSpecializeArgs) {

}

struct AbiWrapper(module_abi<Module>);
unsafe impl Send for AbiWrapper {}
unsafe impl Sync for AbiWrapper {}

static MODULE: SyncOnceCell<Module> = SyncOnceCell::new();
static ABI: SyncOnceCell<AbiWrapper> = SyncOnceCell::new();

#[no_mangle]
pub unsafe extern "C" fn zygisk_module_entry(api: *mut api_table<Module>, env: *mut jni::sys::JNIEnv) {
    let module = Module::new(env);
    if MODULE.set(module).is_err() { return; }

    let abi = module_abi {
        api_version: ZYGISK_API_VERSION,
        impl_: MODULE.get().unwrap() as *const Module as *mut Module,
        preAppSpecialize: Some(pre_app_specialize),
        postAppSpecialize: Some(post_app_specialize),
        preServerSpecialize: Some(pre_server_specialize),
        postServerSpecialize: Some(post_server_specialize)
    };
    if ABI.set(AbiWrapper(abi)).is_err() { return; }

    (*api).registerModule.unwrap()(api, &ABI.get().unwrap().0 as *const _ as *mut _);
}

#[no_mangle]
pub unsafe extern "C" fn zygisk_companion_entry(_arg1: ::std::os::raw::c_int) {

}
