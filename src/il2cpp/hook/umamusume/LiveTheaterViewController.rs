use crate::{core::{utils::notify_error, game::Region, Hachimi}, il2cpp::{symbols::get_method_addr, types::*}};

static mut CHANGELIVE_ONSUCCESS_ADDR: usize = 0;
impl_addr_wrapper_fn!(ChangeLive_onSuccess, CHANGELIVE_ONSUCCESS_ADDR, (), this: *mut Il2CppObject, res: *mut Il2CppObject);

type ChangeLiveFn = extern "C" fn(this: *mut Il2CppObject);
extern "C" fn ChangeLive(this: *mut Il2CppObject) {
    if Hachimi::instance().config.load().live_theater_allow_same_chara {
        if unsafe { CHANGELIVE_ONSUCCESS_ADDR } == 0 {
            return notify_error("BUG: Please turn off 'Live theater allow same chara' \
                and report this to the developer.");
        }
        // As of the current version, res is unused so we can safely pass NULL
        return ChangeLive_onSuccess(this, 0 as _);
    }

    get_orig_fn!(ChangeLive, ChangeLiveFn)(this);
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, LiveTheaterViewController);

    let ChangeLive_addr = get_method_addr(LiveTheaterViewController, c"ChangeLive", 0);

    new_hook!(ChangeLive_addr, ChangeLive);

    let region = &Hachimi::instance().game.region;
    unsafe {
        let known_name = match region {
            &Region::Japan | &Region::Taiwan => c"<ChangeLive>b__48_1",
            _ => c"<ChangeLive>b__41_1", // Global
        };

        CHANGELIVE_ONSUCCESS_ADDR = crate::il2cpp::symbols::get_method_addr_opt(
            LiveTheaterViewController, known_name, 1
        ).unwrap_or(0);

        // The callback is a compiler-generated lambda, so its `b__N_1` ordinal
        // changes whenever the client is recompiled. Fall back to matching the
        // shape instead of the exact name.
        if CHANGELIVE_ONSUCCESS_ADDR == 0 {
            CHANGELIVE_ONSUCCESS_ADDR = find_change_live_on_success(LiveTheaterViewController);
        }
    }
}

/// Finds `<ChangeLive>b__*_1(res)` — the single-argument callback that receives
/// `LiveTheaterLiveStartResponse`.
fn find_change_live_on_success(class: *mut Il2CppClass) -> usize {
    let mut iter: *mut std::ffi::c_void = std::ptr::null_mut();
    loop {
        let method = crate::il2cpp::api::il2cpp_class_get_methods(class, &mut iter);
        if method.is_null() { break; }

        let name = unsafe { std::ffi::CStr::from_ptr((*method).name) }.to_string_lossy();
        if name.starts_with("<ChangeLive>b__")
            && unsafe { (*method).is_generic() } == 0
            && crate::il2cpp::api::il2cpp_method_get_param_count(method) == 1
        {
            return unsafe { (*method).methodPointer };
        }
    }
    0
}