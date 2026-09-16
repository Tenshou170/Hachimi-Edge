use std::{
    ffi::CStr,
    os::raw::{c_char, c_int, c_void},
    sync::atomic::{AtomicBool, AtomicI32, AtomicUsize, Ordering}
};

use log::{error, info, warn};
use jni::sys::{jint, JNINativeMethod, jclass};

use crate::{android::gui_impl::input_hook, core::{Error, Hachimi, Interceptor}};
use super::utils;

const LINKER_MODULE: &str = if cfg!(target_pointer_width = "64") {
    "linker64"
} else {
    "linker"
};

static CACHED_API_LEVEL: AtomicI32 = AtomicI32::new(-1);

pub fn cached_api_level() -> i32 {
    CACHED_API_LEVEL.load(Ordering::Acquire)
}

static DLOPEN_HOOKED_ADDR: AtomicUsize = AtomicUsize::new(0);

type DlopenFn = extern "C" fn(filename: *const c_char, flags: c_int) -> *mut c_void;
extern "C" fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void {
    let hachimi = Hachimi::instance();
    let orig_fn: DlopenFn = unsafe {
        std::mem::transmute(hachimi.interceptor.get_trampoline_addr(dlopen as usize))
    };

    let handle = orig_fn(filename, flags);
    if !hachimi.hooking_finished.load(Ordering::Relaxed) && !filename.is_null() {
        let filename_str = unsafe { CStr::from_ptr(filename).to_string_lossy() };
        if hachimi.on_dlopen(&filename_str, handle as usize) {
            let addr = DLOPEN_HOOKED_ADDR.load(Ordering::Relaxed);
            if addr != 0 {
                hachimi.interceptor.unhook(addr);
            }
        }
    }

    handle
}

type DoDlopenFn = extern "C" fn(filename: *const c_char, flags: c_int, extinfo: *const c_void, caller_addr: *const c_void) -> *mut c_void;
extern "C" fn do_dlopen(filename: *const c_char, flags: c_int, extinfo: *const c_void, caller_addr: *const c_void) -> *mut c_void {
    let hachimi = Hachimi::instance();
    let orig_fn: DoDlopenFn = unsafe {
        std::mem::transmute(hachimi.interceptor.get_trampoline_addr(do_dlopen as usize))
    };

    let handle = orig_fn(filename, flags, extinfo, caller_addr);
    if !hachimi.hooking_finished.load(Ordering::Relaxed) && !filename.is_null() {
        let filename_str = unsafe { CStr::from_ptr(filename).to_string_lossy() };
        if hachimi.on_dlopen(&filename_str, handle as usize) {
            let addr = DLOPEN_HOOKED_ADDR.load(Ordering::Relaxed);
            if addr != 0 {
                hachimi.interceptor.unhook(addr);
            }
        }
    }

    handle
}

type RegisterNativesFn = unsafe extern "system" fn(
    env: *mut *const jni::sys::JNINativeInterface_,
    class: jclass,
    methods: *const JNINativeMethod,
    count: jint,
) -> jint;

static ORIG_REGISTER_NATIVES: AtomicUsize = AtomicUsize::new(0);

#[allow(non_snake_case)]
extern "system" fn JNINativeInterface_RegisterNatives(
    env: *mut *const jni::sys::JNINativeInterface_,
    class: jclass,
    methods_: *const JNINativeMethod,
    count: jint,
) -> jint {
    static GOT_EVENT: AtomicBool = AtomicBool::new(false);
    if !GOT_EVENT.load(Ordering::Relaxed) {
        let methods = unsafe { std::slice::from_raw_parts(methods_, count as usize) };
        for method in methods {
            if method.name.is_null() { continue; }
            let name = unsafe { CStr::from_ptr(method.name).to_string_lossy() };
            if name == "nativeInjectEvent" {
                info!("Got nativeInjectEvent address");
                unsafe {
                    input_hook::NATIVE_INJECT_EVENT_ADDR = method.fnPtr as usize;
                }
                GOT_EVENT.store(true, Ordering::Relaxed);
                break;
            }
        }
    }

    let orig_addr = ORIG_REGISTER_NATIVES.load(Ordering::Acquire);
    if orig_addr != 0 {
        let orig_fn: RegisterNativesFn = unsafe { std::mem::transmute(orig_addr) };
        unsafe { orig_fn(env, class, methods_, count) }
    } else {
        jni::sys::JNI_ERR
    }
}


fn get_page_protection(addr: usize) -> Option<c_int> {
    use std::io::BufRead;
    let file = std::fs::File::open("/proc/self/maps").ok()?;
    let reader = std::io::BufReader::new(file);
    for line in reader.lines().map_while(Result::ok) {
        let mut parts = line.split_whitespace();
        let range = parts.next()?;
        let perms = parts.next()?;
        let mut addrs = range.split('-');
        let start = usize::from_str_radix(addrs.next()?, 16).ok()?;
        let end = usize::from_str_radix(addrs.next()?, 16).ok()?;
        if addr >= start && addr < end {
            let mut prot = 0;
            if perms.contains('r') { prot |= libc::PROT_READ; }
            if perms.contains('w') { prot |= libc::PROT_WRITE; }
            if perms.contains('x') { prot |= libc::PROT_EXEC; }
            return Some(prot);
        }
    }
    None
}

fn init_internal(env: *mut jni::sys::JNIEnv) -> Result<(), Error> {
    let api_level = utils::get_device_api_level(env);
    CACHED_API_LEVEL.store(api_level, Ordering::Release);
    info!("API level: {}", api_level);

    let hachimi = Hachimi::instance();

    let force_hook_dlopen = hachimi.config.load().android.hook_libc_dlopen ||
        std::fs::metadata("/vendor/waydroid.prop").ok().is_some_and(|m| m.is_file());

    let mut dlopen_orig = libc::dlopen as usize;
    let mut dlopen_hook = dlopen as usize;
    let mut dlopen_name = "dlopen";

    const DO_DLOPEN_V24: &str = "__dl__Z9do_dlopenPKciPK17android_dlextinfoPv";  // A7, A7.1
    const DO_DLOPEN_V26: &str = "__dl__Z9do_dlopenPKciPK17android_dlextinfoPKv"; // A8 or later
    if !force_hook_dlopen {
        if api_level >= 26 {
            dlopen_orig = Interceptor::find_symbol_by_name(LINKER_MODULE, DO_DLOPEN_V26)?;
            dlopen_hook = do_dlopen as _;
            dlopen_name = DO_DLOPEN_V26;
        }
        else if api_level >= 24 {
            dlopen_orig = Interceptor::find_symbol_by_name(LINKER_MODULE, DO_DLOPEN_V24)?;
            dlopen_hook = do_dlopen as _;
            dlopen_name = DO_DLOPEN_V24;
        }
    }

    info!("Hooking {} at {:#x}", dlopen_name, dlopen_orig);
    hachimi.interceptor.hook(dlopen_orig, dlopen_hook)?;
    DLOPEN_HOOKED_ADDR.store(dlopen_orig, Ordering::Relaxed);

    if !hachimi.config.load().disable_gui && ORIG_REGISTER_NATIVES.load(Ordering::Acquire) == 0 {
        info!("Hooking JNINativeInterface RegisterNatives via JNI vtable");
        unsafe {
            let jni_table = *env as *mut jni::sys::JNINativeInterface_;
            let orig = (*jni_table).RegisterNatives;
            ORIG_REGISTER_NATIVES.store(orig.map(|f| f as usize).unwrap_or(0), Ordering::Release);

            // The field being written can land on a different page than the one
            // containing the start of the table (depends on how the vendor's ART
            // build laid out the const table in .rodata), so the mprotect range
            // must be derived from the field's own address/size, not the table's.
            let field_addr = std::ptr::addr_of!((*jni_table).RegisterNatives) as usize;
            let field_size = std::mem::size_of::<Option<RegisterNativesFn>>();

            let orig_prot = get_page_protection(field_addr);
            let is_already_writable = orig_prot.map(|p| (p & libc::PROT_WRITE) != 0).unwrap_or(false);

            if is_already_writable {
                // Table is already on a writable page (common in modern ART on Android 15+ / 17).
                // Do NOT call mprotect, which would strip write permissions on co-located ART heap structures.
                (*jni_table).RegisterNatives = Some(JNINativeInterface_RegisterNatives);
            } else {
                let page_size = libc::sysconf(libc::_SC_PAGESIZE) as usize;
                let range_start = field_addr & !(page_size - 1);
                let range_end = (field_addr + field_size + page_size - 1) & !(page_size - 1);
                let range_len = range_end - range_start;

                let prot_to_set = orig_prot.unwrap_or(libc::PROT_READ) | libc::PROT_WRITE;
                let ret = libc::mprotect(
                    range_start as *mut c_void,
                    range_len,
                    prot_to_set,
                );
                if ret != 0 {
                    let errno = std::io::Error::last_os_error();
                    warn!(
                        "mprotect(RW) on JNI vtable page(s) failed ({}); \
                         RegisterNatives hook skipped — nativeInjectEvent won't be intercepted",
                        errno
                    );

                    ORIG_REGISTER_NATIVES.store(0, Ordering::Release);
                    return Ok(());
                }

                (*jni_table).RegisterNatives = Some(JNINativeInterface_RegisterNatives);

                // Restore only the original protection if we knew it was non-writable.
                if let Some(prot) = orig_prot {
                    let ret = libc::mprotect(range_start as *mut c_void, range_len, prot);
                    if ret != 0 {
                        let errno = std::io::Error::last_os_error();
                        warn!("mprotect restore on JNI vtable page(s) failed ({})", errno);
                    }
                }
            }
        }
    }

    Ok(())
}

pub fn init(env: *mut jni::sys::JNIEnv) {
    init_internal(env).unwrap_or_else(|e| error!("Init failed: {}", e));
}
