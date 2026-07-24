use std::ffi::{CStr, CString};
use std::os::raw::c_void;
use std::ptr::null_mut;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use once_cell::sync::Lazy;

use crate::il2cpp::api::*;
use crate::il2cpp::ext::Il2CppObjectExt;
use crate::il2cpp::symbols;
use crate::il2cpp::types::*;

// Cached main thread pointer — resolved once on first caption show and reused
// for all subsequent fade-tick schedules, avoiding per-tick attached_threads()
// calls (which call il2cpp_thread_get_all_attached_threads) during the 60 Hz
// fade loop.
static MAIN_THREAD_PTR: AtomicUsize = AtomicUsize::new(0);

fn get_main_thread() -> Option<symbols::Thread> {
    let cached = MAIN_THREAD_PTR.load(Ordering::Relaxed);
    if cached != 0 {
        return Some(symbols::Thread::from_raw(cached as *mut Il2CppThread));
    }
    let thread = symbols::Thread::attached_threads().first().cloned()?;
    MAIN_THREAD_PTR.store(thread.as_raw() as usize, Ordering::Relaxed);
    Some(thread)
}

struct CaptionState {
    handle: Option<symbols::GCHandle>,
    inited: bool,
    fade_id: u64,
    fade_start_time: Option<std::time::Instant>,
    display_time: f32,
    fade_out_time: f32,
}

#[derive(Clone)]
struct CaptionSnapshot {
    font_size: i32,
    font_color: String,
    outline_size: String,
    outline_color: String,
    pos_x: f32,
    pos_y: f32,
    bg_alpha: f32,
}

impl CaptionState {
    fn notification(&self) -> *mut Il2CppObject {
        self.handle.as_ref().map_or(null_mut(), |h| h.target())
    }

    fn clear(&mut self) {
        self.handle = None;
        self.inited = false;
        self.fade_id = self.fade_id.wrapping_add(1);
    }

    fn set_notification(&mut self, obj: *mut Il2CppObject) {
        self.handle = None;
        self.fade_id = self.fade_id.wrapping_add(1);
        if !obj.is_null() {
            self.handle = Some(symbols::GCHandle::new(obj, false));
        }
    }
}

static STATE: Lazy<Mutex<CaptionState>> = Lazy::new(|| {
    Mutex::new(CaptionState {
        handle: None,
        inited: false,
        fade_id: 0,
        fade_start_time: None,
        display_time: 0.0,
        fade_out_time: 0.5,
    })
});

fn is_native_alive(obj: *mut Il2CppObject) -> bool {
    if obj.is_null() { return false; }
    crate::il2cpp::hook::UnityEngine_CoreModule::Object::IsNativeObjectAlive(obj)
}

fn invoke(method: *const MethodInfo, obj: *mut c_void, params: *mut *mut c_void) -> *mut Il2CppObject {
    if method.is_null() { return null_mut(); }
    let mut exc: *mut Il2CppException = null_mut();
    let r = il2cpp_runtime_invoke(method, obj, params, &mut exc);
    if !exc.is_null() { return null_mut(); }
    r
}

fn invoke_method(klass: *mut Il2CppClass, name: &std::ffi::CStr, argc: i32, obj: *mut c_void, params: *mut *mut c_void) -> *mut Il2CppObject {
    let m = il2cpp_class_get_method_from_name(klass, name.as_ptr(), argc);
    invoke(m, obj, params)
}

fn get_class(asm: &std::ffi::CStr, ns: &std::ffi::CStr, name: &std::ffi::CStr) -> *mut Il2CppClass {
    let image = match symbols::get_assembly_image(asm) {
        Ok(img) => img,
        Err(_) => return null_mut(),
    };
    match symbols::get_class(image, ns, name) {
        Ok(c) => c,
        Err(_) => null_mut(),
    }
}

fn get_runtime_type(asm: &CStr, ns: &CStr, name: &CStr) -> *mut Il2CppObject {
    let k = get_class(asm, ns, name);
    if k.is_null() { return null_mut(); }
    let t = il2cpp_class_get_type(k);
    if t.is_null() { return null_mut(); }
    il2cpp_type_get_object(t) as *mut Il2CppObject
}

fn klass(obj: *mut Il2CppObject) -> *mut Il2CppClass {
    if obj.is_null() { null_mut() } else { unsafe { (*obj).klass() } }
}

fn invoke_method_on(obj: *mut Il2CppObject, name: &CStr, argc: i32, params: *mut *mut c_void) -> *mut Il2CppObject {
    let k = klass(obj);
    if k.is_null() { return null_mut(); }
    invoke_method(k, name, argc, obj as _, params)
}

fn parse_enum(enum_type: *mut Il2CppObject, value: &str) -> *mut Il2CppObject {
    if enum_type.is_null() || value.is_empty() { return null_mut(); }
    let enum_class = get_class(c"mscorlib.dll", c"System", c"Enum");
    if enum_class.is_null() { return null_mut(); }
    let c_val = match CString::new(value) { Ok(v) => v, Err(_) => return null_mut() };
    let val_str = il2cpp_string_new(c_val.as_ptr());
    let mut params: [*mut c_void; 2] = [enum_type as _, val_str as _];
    invoke_method(enum_class, c"Parse", 2, null_mut(), params.as_mut_ptr())
}

fn get_enum_int(e: *mut Il2CppObject) -> i32 {
    if e.is_null() { return 0; }
    let enum_class = get_class(c"mscorlib.dll", c"System", c"Enum");
    if enum_class.is_null() { return 0; }
    let mut params: [*mut c_void; 1] = [e as _];
    let r = invoke_method(enum_class, c"ToUInt64", 1, null_mut(), params.as_mut_ptr());
    if r.is_null() { return 0; }
    unsafe { *(il2cpp_object_unbox(r) as *mut u64) as i32 }
}

// Mark call sites that read the method pointer as unsafe, so callers
// must explicitly handle safety. The function itself is unchanged.
unsafe fn method_pointer(m: *const MethodInfo) -> usize {
    if m.is_null() { return 0; }
    *(m as *const usize)
}

// On Windows, use microseh to catch structured exceptions; on other
// targets, use catch_unwind to recover from Rust panics and reset state.
#[cfg(target_os = "windows")]
fn guarded<F: FnMut()>(mut f: F) {
    if microseh::try_seh(|| f()).is_err() {
        warn!("[captions] SEH exception caught, resetting state");
        if let Ok(mut st) = STATE.lock() { st.clear(); }
    }
}

#[cfg(not(target_os = "windows"))]
fn guarded<F: FnMut() + std::panic::UnwindSafe>(mut f: F) {
    // catch_unwind only catches Rust panics, not SIGSEGV.  All _impl
    // functions must guard every raw pointer before use so that a SIGSEGV
    // is never reachable.
    if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f())).is_err() {
        warn!("[captions] panic caught, resetting state");
        if let Ok(mut st) = STATE.lock() { st.clear(); }
    }
}

// Helper to lock STATE without panicking on poison.
macro_rules! state_lock {
    () => {
        match STATE.lock() {
            Ok(g) => g,
            Err(e) => {
                warn!("[captions] STATE mutex poisoned: {}", e);
                return;
            }
        }
    };
}

fn init_impl() {
    // Avoid reinitializing while the current notification is alive.
    let mut st = state_lock!();
    let notif = st.notification();
    if st.inited && !notif.is_null() && is_native_alive(notif) { return; }
    st.clear();

    let ui_mgr_class = get_class(c"umamusume.dll", c"Gallop", c"UIManager");
    if ui_mgr_class.is_null() { return; }
    let ui_mgr = invoke_method(ui_mgr_class, c"get_Instance", 0, null_mut(), null_mut());
    if ui_mgr.is_null() { return; }

    let mut canvas: *mut Il2CppObject = null_mut();
    for fname in [c"_noticeCanvas", c"_systemCanvas", c"_mainCanvas"] {
        let f = il2cpp_class_get_field_from_name(ui_mgr_class, fname.as_ptr());
        if !f.is_null() {
            il2cpp_field_get_value(ui_mgr, f, &mut canvas as *mut _ as _);
            if !canvas.is_null() { break; }
        }
    }
    if canvas.is_null() { return; }

    let transform = invoke_method_on(canvas, c"get_transform", 0, null_mut());
    if transform.is_null() { return; }

    let res_class = get_class(c"UnityEngine.CoreModule.dll", c"UnityEngine", c"Resources");
    if res_class.is_null() { return; }
    let path = il2cpp_string_new(c"UI/Parts/Notification".as_ptr());
    let go_type = get_runtime_type(c"UnityEngine.CoreModule.dll", c"UnityEngine", c"GameObject");
    if go_type.is_null() { return; }
    let mut load_params: [*mut c_void; 2] = [path as _, go_type as _];
    let prefab = invoke_method(res_class, c"Load", 2, null_mut(), load_params.as_mut_ptr());
    if prefab.is_null() { return; }

    type CloneFn = extern "C" fn(*mut Il2CppObject, *mut Il2CppObject, bool) -> *mut Il2CppObject;
    let clone_fn: CloneFn = unsafe {
        let ptr = il2cpp_resolve_icall(c"UnityEngine.Object::Internal_CloneSingleWithParent()".as_ptr());
        if ptr == 0 { return; }
        std::mem::transmute(ptr)
    };
    let inst = clone_fn(prefab, transform, false);
    if inst.is_null() { return; }

    let notif_type = get_runtime_type(c"umamusume.dll", c"Gallop", c"Notification");
    if notif_type.is_null() { return; }
    let mut inc_inactive: bool = true;
    let mut gc_params: [*mut c_void; 2] = [notif_type as _, &mut inc_inactive as *mut bool as _];
    let go_class = get_class(c"UnityEngine.CoreModule.dll", c"UnityEngine", c"GameObject");
    if go_class.is_null() { return; }
    let new_notif = invoke_method(go_class, c"GetComponentInChildren", 2, inst as _, gc_params.as_mut_ptr());
    if new_notif.is_null() { return; }
    st.set_notification(new_notif);

    let go = invoke_method_on(new_notif, c"get_gameObject", 0, null_mut());
    if !go.is_null() {
        let mut active: bool = false;
        let mut p: [*mut c_void; 1] = [&mut active as *mut bool as _];
        invoke_method_on(go, c"SetActive", 1, p.as_mut_ptr());
        st.inited = true;
    }
    if !st.inited { st.clear(); }
}

fn snapshot_caption_config() -> CaptionSnapshot {
    let cfg = crate::core::Hachimi::instance().config.load();
    CaptionSnapshot {
        font_size: cfg.caption.caption_font_size,
        font_color: cfg.caption.caption_color.clone(),
        outline_size: cfg.caption.caption_outline_size.clone(),
        outline_color: cfg.caption.caption_outline_color.clone(),
        pos_x: cfg.caption.caption_pos_x,
        pos_y: cfg.caption.caption_pos_y,
        bg_alpha: cfg.caption.caption_bg_alpha,
    }
}

fn show_impl(text: &str, line_char_count: i32) {
    // Snapshot notif and nk while holding the lock, then re-validate
    // liveness after dropping it before any il2cpp call.
    let (notif, nk) = {
        // Avoid panics on poisoned STATE lock.
        let mut st = state_lock!();
        let notif = st.notification();
        if notif.is_null() || !is_native_alive(notif) {
            st.clear();
            return;
        }
        let nk = klass(notif);
        (notif, nk)
        // lock dropped here
    };
    // Re-validate after dropping the lock, in case cleanup_impl ran.
    if !is_native_alive(notif) { return; }

    let label_f = il2cpp_class_get_field_from_name(nk, c"_Label".as_ptr());
    let cg_f    = il2cpp_class_get_field_from_name(nk, c"canvasGroup".as_ptr());
    if label_f.is_null() || cg_f.is_null() { return; }

    let mut label: *mut Il2CppObject = null_mut();
    let mut cg:    *mut Il2CppObject = null_mut();
    il2cpp_field_get_value(notif, label_f, &mut label as *mut _ as _);
    il2cpp_field_get_value(notif, cg_f,    &mut cg    as *mut _ as _);
    if label.is_null() || cg.is_null() { return; }

    let c_text   = match CString::new(text) { Ok(v) => v, Err(_) => return };
    let mut il2_text = il2cpp_string_new(c_text.as_ptr()) as *mut Il2CppObject;

    if line_char_count > 0 {
        let gu_class = get_class(c"umamusume.dll", c"Gallop", c"GallopUtil");
        if !gu_class.is_null() {
            let mut lcc = line_char_count;
            let mut p: [*mut c_void; 2] = [il2_text as _, &mut lcc as *mut i32 as _];
            let wrapped = invoke_method(gu_class, c"LineHeadWrap", 2, null_mut(), p.as_mut_ptr());
            if !wrapped.is_null() {
                il2_text = wrapped;
            }
        }
    }

    unsafe {
        let set_text_fp = method_pointer(il2cpp_class_get_method_from_name(klass(label), c"set_text".as_ptr(), 1));
        if set_text_fp != 0 {
            let set_text: extern "C" fn(*mut Il2CppObject, *mut Il2CppObject) = std::mem::transmute(set_text_fp);
            set_text(label, il2_text);
        }

        let set_alpha_fp = method_pointer(il2cpp_class_get_method_from_name(klass(cg), c"set_alpha".as_ptr(), 1));
        if set_alpha_fp != 0 {
            let set_alpha: extern "C" fn(*mut Il2CppObject, f32) = std::mem::transmute(set_alpha_fp);
            set_alpha(cg, 1.0);
        }

        let go_fp = method_pointer(il2cpp_class_get_method_from_name(nk, c"get_gameObject".as_ptr(), 0));
        if go_fp != 0 {
            let get_go: extern "C" fn(*mut Il2CppObject) -> *mut Il2CppObject = std::mem::transmute(go_fp);
            let go = get_go(notif);
            if !go.is_null() {
                let sa_fp = method_pointer(il2cpp_class_get_method_from_name(klass(go), c"SetActive".as_ptr(), 1));
                if sa_fp != 0 {
                    let set_active: extern "C" fn(*mut Il2CppObject, bool) = std::mem::transmute(sa_fp);
                    set_active(go, true);
                }
            }
        }
    }

    let snap = snapshot_caption_config();
    set_format_impl(
        snap.font_size,
        &snap.font_color,
        &snap.outline_size,
        &snap.outline_color,
        snap.pos_x,
        snap.pos_y,
        snap.bg_alpha,
    );

    let mut display_time: f32 = 0.0;
    let mut fade_out:     f32 = 0.5;
    let dt_f = il2cpp_class_get_field_from_name(nk, c"_displayTime".as_ptr());
    let fo_f = il2cpp_class_get_field_from_name(nk, c"_fadeOutTime".as_ptr());
    if !dt_f.is_null() { il2cpp_field_get_value(notif, dt_f, &mut display_time as *mut f32 as _); }
    if !fo_f.is_null() { il2cpp_field_get_value(notif, fo_f, &mut fade_out     as *mut f32 as _); }

    {
        // Update fade state atomically while holding the lock.
        let mut st = state_lock!();
        st.fade_id = st.fade_id.wrapping_add(1);
        st.fade_start_time = Some(std::time::Instant::now());
        st.display_time  = display_time;
        st.fade_out_time = fade_out;
    }

    // Schedule fade tick on the attached main thread if available.
    if let Some(main) = get_main_thread() {
        main.schedule(fade_tick_global);
    } else {
        warn!("[captions] no attached threads, fade tick not scheduled");
    }
}

fn fade_tick_global() {
    // Snapshot under lock and re-validate after drop.
    let (notif, nk, start_time, display_time, fade_out) = {
        // Ensure STATE lock is handled safely.
        let st = state_lock!();
        let notif = st.notification();
        if notif.is_null() || !is_native_alive(notif) { return; }
        let start_time = match st.fade_start_time { Some(t) => t, None => return };
        let nk = klass(notif);
        (notif, nk, start_time, st.display_time, st.fade_out_time)
        // lock dropped here
    };
    if !is_native_alive(notif) { return; }

    let elapsed = start_time.elapsed().as_secs_f32();
    let mut alpha  = 1.0f32;
    let mut active = true;
    let mut done   = false;

    if elapsed >= display_time + fade_out {
        alpha  = 0.0;
        active = false;
        done   = true;
    } else if elapsed >= display_time {
        let progress = (elapsed - display_time) / fade_out.max(0.001);
        alpha = 1.0 - progress.clamp(0.0, 1.0);
    }

    let cg_f = il2cpp_class_get_field_from_name(nk, c"canvasGroup".as_ptr());
    if !cg_f.is_null() {
        let mut cg: *mut Il2CppObject = null_mut();
        il2cpp_field_get_value(notif, cg_f, &mut cg as *mut _ as _);
        if !cg.is_null() {
            unsafe {
                let set_alpha_fp = method_pointer(il2cpp_class_get_method_from_name(klass(cg), c"set_alpha".as_ptr(), 1));
                if set_alpha_fp != 0 {
                    let set_alpha: extern "C" fn(*mut Il2CppObject, f32) = std::mem::transmute(set_alpha_fp);
                    set_alpha(cg, alpha);
                }
            }
        }
    }

    if !active {
        unsafe {
            let go_fp = method_pointer(il2cpp_class_get_method_from_name(nk, c"get_gameObject".as_ptr(), 0));
            if go_fp != 0 {
                let get_go: extern "C" fn(*mut Il2CppObject) -> *mut Il2CppObject = std::mem::transmute(go_fp);
                let go = get_go(notif);
                if !go.is_null() {
                    let sa_fp = method_pointer(il2cpp_class_get_method_from_name(klass(go), c"SetActive".as_ptr(), 1));
                    if sa_fp != 0 {
                        let set_active: extern "C" fn(*mut Il2CppObject, bool) = std::mem::transmute(sa_fp);
                        set_active(go, false);
                    }
                }
            }
        }
    }

    if !done {
        // Schedule the next fade tick on the main thread.
        if let Some(main) = get_main_thread() {
            main.schedule(fade_tick_global);
        }
    }
}

fn set_display_time_impl(time: f32) {
    // Snapshot under lock and re-validate after dropping it.
    let (notif, nk) = {
        // Avoid mutex poison panics by handling a poisoned STATE lock.
        let st = state_lock!();
        let notif = st.notification();
        if notif.is_null() || !is_native_alive(notif) { return; }
        let nk = klass(notif);
        (notif, nk)
    };
    if !is_native_alive(notif) { return; }

    let f = il2cpp_class_get_field_from_name(nk, c"_displayTime".as_ptr());
    if !f.is_null() {
        il2cpp_field_set_value(notif, f, &time as *const f32 as _);
    }
}

fn set_format_impl(
    font_size: i32,
    font_color: &str,
    outline_size: &str,
    outline_color: &str,
    pos_x: f32,
    pos_y: f32,
    bg_alpha: f32,
) {
    // Snapshot under lock and re-validate after dropping it.
    let (notif, nk) = {
        // Avoid mutex poison panics by handling a poisoned STATE lock.
        let st = state_lock!();
        let notif = st.notification();
        if notif.is_null() || !is_native_alive(notif) { return; }
        let nk = klass(notif);
        (notif, nk)
    };
    if !is_native_alive(notif) { return; }

    let label_f = il2cpp_class_get_field_from_name(nk, c"_Label".as_ptr());
    if label_f.is_null() { return; }
    let mut label: *mut Il2CppObject = null_mut();
    il2cpp_field_get_value(notif, label_f, &mut label as *mut _ as _);
    if label.is_null() { return; }
    let lk = klass(label);

    // Fetch screen dimensions early, reuse throughout
    let mut screen_width = 1080;
    let mut screen_height = 1920;
    let screen_class = get_class(c"UnityEngine.CoreModule.dll", c"UnityEngine", c"Screen");
    if !screen_class.is_null() {
        let w_obj = invoke_method(screen_class, c"get_width", 0, null_mut(), null_mut());
        if !w_obj.is_null() {
            screen_width = unsafe { *(il2cpp_object_unbox(w_obj) as *mut i32) };
        }
        let h_obj = invoke_method(screen_class, c"get_height", 0, null_mut(), null_mut());
        if !h_obj.is_null() {
            screen_height = unsafe { *(il2cpp_object_unbox(h_obj) as *mut i32) };
        }
    }

    if Captions::format_log_enabled() {
        info!("[captions] formatting | requested | font_size={} font_color={} outline_size={} outline_color={} pos_x={} pos_y={} bg_alpha={}",
            font_size, font_color, outline_size, outline_color, pos_x, pos_y, bg_alpha);
        // Try reading current font size before modification
        let current_fs = crate::il2cpp::hook::UnityEngine_UI::Text::get_fontSize(label);
        info!("[captions] formatting | before set_fontSize current_font_size={}", current_fs);
    }

    let target_width  = (font_size as f32 * 30.0).max(1000.0);
    let target_height = (font_size as f32 * 6.0).max(300.0);

    if Captions::format_log_enabled() {
        info!("[captions] formatting | computed target_width={} target_height={}", target_width, target_height);
    }

    let label_tr = invoke_method(lk, c"get_transform", 0, label as _, null_mut());
    if !label_tr.is_null() {
        use crate::il2cpp::hook::UnityEngine_CoreModule::{RectTransform, Transform};
        
        // Allow text to wrap within generous bounds
        let label_width = target_width.min(screen_width as f32 * 0.9);
        let label_height = target_height.min(screen_height as f32 * 0.7);
        RectTransform::set_sizeDelta(label_tr, Vector2_t { x: label_width, y: label_height });
        let label_scale = Transform::get_localScale(label_tr);

        if Captions::format_log_enabled() {
            info!("[captions] formatting | set_sizeDelta for label: width={} height={} (screen: {}x{}) localScale=({}, {}, {})",
                label_width, label_height, screen_width, screen_height, label_scale.x, label_scale.y, label_scale.z);
        }
    }

    let mut wrap: i32 = 0;
    let mut wp: [*mut c_void; 1] = [&mut wrap as *mut i32 as _];
    invoke_method(lk, c"set_horizontalOverflow", 1, label as _, wp.as_mut_ptr());

    let mut vertical_overflow: i32 = 1;
    let mut vp: [*mut c_void; 1] = [&mut vertical_overflow as *mut i32 as _];
    invoke_method(lk, c"set_verticalOverflow", 1, label as _, vp.as_mut_ptr());

    let mut best_fit_on: bool = false;
    let mut bfp: [*mut c_void; 1] = [&mut best_fit_on as *mut bool as _];
    invoke_method(lk, c"set_resizeTextForBestFit", 1, label as _, bfp.as_mut_ptr());

    let mut min_size: i32 = font_size;
    let mut min_sp: [*mut c_void; 1] = [&mut min_size as *mut i32 as _];
    invoke_method(lk, c"set_resizeTextMinSize", 1, label as _, min_sp.as_mut_ptr());

    if Captions::format_log_enabled() {
        info!("[captions] formatting | set_resizeTextForBestFit={} resizeTextMinSize={}", best_fit_on, min_size);
    }

    let mut fs = font_size;
    let mut sp: [*mut c_void; 1] = [&mut fs as *mut i32 as _];
    invoke_method(lk, c"set_fontSize",          1, label as _, sp.as_mut_ptr());
    invoke_method(lk, c"set_resizeTextMaxSize", 1, label as _, sp.as_mut_ptr());

    if Captions::format_log_enabled() {
        // Read back font size from the Text component to verify it took effect
        let observed = crate::il2cpp::hook::UnityEngine_UI::Text::get_fontSize(label);
        info!("[captions] formatting | after set_fontSize observed_font_size={} set_resizeTextMaxSize={}", observed, fs);
    }

    if !font_color.is_empty() {
        let e = parse_enum(get_runtime_type(c"umamusume.dll", c"Gallop", c"FontColorType"), font_color);
        if !e.is_null() {
            let mut v = get_enum_int(e);
            let mut p: [*mut c_void; 1] = [&mut v as *mut i32 as _];
            invoke_method(lk, c"set_FontColor", 1, label as _, p.as_mut_ptr());
        }
    }

    if !outline_size.is_empty() {
        let e = parse_enum(get_runtime_type(c"umamusume.dll", c"Gallop", c"OutlineSizeType"), outline_size);
        if !e.is_null() {
            let mut v = get_enum_int(e);
            let mut p: [*mut c_void; 1] = [&mut v as *mut i32 as _];
            invoke_method(lk, c"set_OutlineSize", 1, label as _, p.as_mut_ptr());
        }
        invoke_method(lk, c"UpdateOutline", 0, label as _, null_mut());
    }

    if !outline_color.is_empty() {
        let e = parse_enum(get_runtime_type(c"umamusume.dll", c"Gallop", c"OutlineColorType"), outline_color);
        if !e.is_null() {
            let mut v = get_enum_int(e);
            let mut p: [*mut c_void; 1] = [&mut v as *mut i32 as _];
            invoke_method(lk, c"set_OutlineColor", 1, label as _, p.as_mut_ptr());
        }
        invoke_method(lk, c"RebuildOutline", 0, label as _, null_mut());
    }

    let go = invoke_method(nk, c"get_gameObject", 0, notif as _, null_mut());
    if !go.is_null() {
        let img_type = get_runtime_type(c"umamusume.dll", c"Gallop", c"ImageCommon");
        if !img_type.is_null() {
            let mut inc: bool = true;
            let mut bgp: [*mut c_void; 2] = [img_type as _, &mut inc as *mut bool as _];
            let bg = invoke_method_on(go, c"GetComponentInChildren", 2, bgp.as_mut_ptr());
            if !bg.is_null() {
                let bg_k = klass(bg);
                let mut ba = bg_alpha;
                let mut p: [*mut c_void; 1] = [&mut ba as *mut f32 as _];
                invoke_method(bg_k, c"SetAlpha", 1, bg as _, p.as_mut_ptr());

                let bg_tr = invoke_method(bg_k, c"get_transform", 0, bg as _, null_mut());
                if !bg_tr.is_null() {
                    use crate::il2cpp::hook::UnityEngine_CoreModule::{RectTransform, Transform};
                    let bg_width = (target_width + 50.0).min(screen_width as f32 * 0.95);
                    let bg_height = (target_height + 20.0).min(screen_height as f32 * 0.5);
                    RectTransform::set_sizeDelta(bg_tr, Vector2_t { x: bg_width, y: bg_height });
                    let bg_scale = Transform::get_localScale(bg_tr);

                    if Captions::format_log_enabled() {
                        info!("[captions] formatting | set_sizeDelta for bg: width={} height={} localScale=({}, {}, {})", bg_width, bg_height, bg_scale.x, bg_scale.y, bg_scale.z);
                    }
                }
            }
        }
    }

    let cg_f = il2cpp_class_get_field_from_name(nk, c"canvasGroup".as_ptr());
    if cg_f.is_null() { return; }
    let mut cg: *mut Il2CppObject = null_mut();
    il2cpp_field_get_value(notif, cg_f, &mut cg as *mut _ as _);
    if cg.is_null() || !is_native_alive(cg) { return; }

    let cg_tr = invoke_method_on(cg, c"get_transform", 0, null_mut());
    if cg_tr.is_null() { return; }
    let tr_k = klass(cg_tr);

    use crate::il2cpp::hook::UnityEngine_CoreModule::{RectTransform, Transform};
    
    // Scale container to fit screen bounds
    let clamped_width = (target_width + 100.0).min(screen_width as f32 * 0.95);
    let clamped_height = (target_height + 50.0).min(screen_height as f32 * 0.5);
    RectTransform::set_sizeDelta(cg_tr, Vector2_t { x: clamped_width, y: clamped_height });
    let cg_scale = Transform::get_localScale(cg_tr);

    if Captions::format_log_enabled() {
        info!("[captions] formatting | set_sizeDelta for cg_tr: width={} height={} (clamped to screen: {}x{}) localScale=({}, {}, {})", clamped_width, clamped_height, screen_width, screen_height, cg_scale.x, cg_scale.y, cg_scale.z);
    }

    let get_pos_m = il2cpp_class_get_method_from_name(tr_k, c"get_position".as_ptr(), 0);
    let set_pos_m = il2cpp_class_get_method_from_name(tr_k, c"set_position".as_ptr(), 1);
    if !get_pos_m.is_null() && !set_pos_m.is_null() {
        let pos_obj = invoke(get_pos_m, cg_tr as _, null_mut());
        if !pos_obj.is_null() {
            #[repr(C)]
            #[derive(Clone, Copy)]
            struct Vec3 { x: f32, y: f32, z: f32 }

            // Get screen orientation
            let is_landscape = screen_width > screen_height;
            let final_pos_y = if is_landscape {
                pos_y * 0.55
            } else {
                pos_y
            };

            if Captions::format_log_enabled() {
                info!("[captions] formatting | screen for position: width={} height={} landscape={}", screen_width, screen_height, is_landscape);
            }

            let pos = unsafe { &*(il2cpp_object_unbox(pos_obj) as *const Vec3) };
            let mut new_pos = Vec3 { x: pos_x, y: final_pos_y, z: pos.z };
            let mut p: [*mut c_void; 1] = [&mut new_pos as *mut Vec3 as _];
            invoke(set_pos_m, cg_tr as _, p.as_mut_ptr());
        }
    }
}

#[cfg(test)]
fn insert_soft_breaks(s: &str, max: usize) -> String {
    if max == 0 { return s.to_owned(); }
    let mut out = String::with_capacity(s.len());
    let mut run = 0usize;
    for ch in s.chars() {
        out.push(ch);
        if ch.is_whitespace() {
            run = 0;
        } else {
            run += 1;
            if run >= max {
                // Insert zero-width space as a soft break opportunity
                out.push('\u{200B}');
                run = 0;
            }
        }
    }
    out
}

fn cleanup_impl() {
    // Snapshot notif and invalidate fade id while holding STATE.
    let (notif, nk) = {
        // Avoid panics on poisoned STATE lock.
        let mut st = state_lock!();
        let notif = st.notification();
        if notif.is_null() || !is_native_alive(notif) { return; }
        let nk = klass(notif);
        st.fade_id = st.fade_id.wrapping_add(1);
        (notif, nk)
    };
    if !is_native_alive(notif) { return; }

    let cg_f = il2cpp_class_get_field_from_name(nk, c"canvasGroup".as_ptr());
    if !cg_f.is_null() {
        let mut cg: *mut Il2CppObject = null_mut();
        il2cpp_field_get_value(notif, cg_f, &mut cg as *mut _ as _);
        if !cg.is_null() {
            unsafe {
                let set_alpha_fp = method_pointer(il2cpp_class_get_method_from_name(klass(cg), c"set_alpha".as_ptr(), 1));
                if set_alpha_fp != 0 {
                    let set_alpha: extern "C" fn(*mut Il2CppObject, f32) = std::mem::transmute(set_alpha_fp);
                    set_alpha(cg, 0.0);
                }
            }
        }
    }

    unsafe {
        let go_fp = method_pointer(il2cpp_class_get_method_from_name(nk, c"get_gameObject".as_ptr(), 0));
        if go_fp != 0 {
            let get_go: extern "C" fn(*mut Il2CppObject) -> *mut Il2CppObject = std::mem::transmute(go_fp);
            let go = get_go(notif);
            if !go.is_null() {
                let sa_fp = method_pointer(il2cpp_class_get_method_from_name(klass(go), c"SetActive".as_ptr(), 1));
                if sa_fp != 0 {
                    let set_active: extern "C" fn(*mut Il2CppObject, bool) = std::mem::transmute(sa_fp);
                    set_active(go, false);
                }
            }
        }
    }
}

pub struct Captions;

impl Captions {
    pub fn init() {
        guarded(init_impl);
    }

    pub fn show_log_enabled() -> bool {
        crate::core::Hachimi::instance().config.load().caption.caption_show_log_enable
    }

    pub fn format_log_enabled() -> bool {
        crate::core::Hachimi::instance().config.load().caption.caption_format_log_enable
    }

    pub fn show(text: &str) {
        let text = text.to_owned();
        guarded(move || show_impl(&text, 0));
    }

    pub fn show_wrapped(text: &str, max_chars_per_line: i32) {
        let max = max_chars_per_line.max(0);
        let text = text.to_owned();
        guarded(move || show_impl(&text, max));
    }

    pub fn set_display_time(time: f32) {
        guarded(move || set_display_time_impl(time));
    }

    pub fn set_format(
        font_size: i32,
        font_color: &str,
        outline_size: &str,
        outline_color: &str,
        pos_x: f32,
        pos_y: f32,
        bg_alpha: f32,
    ) {
        let fc = font_color.to_owned();
        let os = outline_size.to_owned();
        let oc = outline_color.to_owned();
        guarded(move || set_format_impl(font_size, &fc, &os, &oc, pos_x, pos_y, bg_alpha));
    }

    pub fn reposition() {
        let snap = snapshot_caption_config();
        Self::set_format(
            snap.font_size,
            &snap.font_color,
            &snap.outline_size,
            &snap.outline_color,
            snap.pos_x,
            snap.pos_y,
            snap.bg_alpha,
        );
    }

    pub fn reposition_scheduled() {
        if let Some(main) = get_main_thread() {
            main.schedule(Self::reposition_callback);
        } else {
            warn!("[captions] no attached threads for reposition scheduling");
        }
    }

    fn reposition_callback() {
        Self::reposition();
    }

    pub fn cleanup() {
        guarded(cleanup_impl);
    }

    pub fn reset() {
        if let Ok(mut st) = STATE.lock() { st.clear(); }
    }
}

#[cfg(test)]
mod tests {
    use super::insert_soft_breaks;

    #[test]
    fn breaks_long_unbroken_run() {
        let s = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"; // 32 a's
        let out = insert_soft_breaks(s, 8);
        // Expect zero-width spaces inserted roughly every 8 chars -> at least 3
        let count = out.matches('\u{200B}').count();
        assert!(count >= 3, "expected >=3 soft breaks, got {}", count);
        // Removing ZWSP should yield original
        let removed: String = out.chars().filter(|&c| c != '\u{200B}').collect();
        assert_eq!(removed, s);
    }

    #[test]
    fn preserves_spaces_and_resets_counter() {
        let s = "aaaaaaaa aaaaaaaa aaaaaaaa"; // spaces should reset
        let out = insert_soft_breaks(s, 8);
        // There should be no ZWSP immediately after a space
        for (i, ch) in out.chars().enumerate() {
            if ch == ' ' {
                let next = out.chars().nth(i+1);
                if let Some(n) = next {
                    assert_ne!(n, '\u{200B}');
                }
            }
        }
    }

    #[test]
    fn zero_max_returns_original() {
        let s = "hello world";
        assert_eq!(insert_soft_breaks(s, 0), s.to_owned());
    }
}
