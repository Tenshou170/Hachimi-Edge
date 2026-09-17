use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::il2cpp::{
    hook::{
        UnityEngine_CoreModule::{Component, GameObject},
        UnityEngine_UI::Text,
        UnityEngine_UIModule::CanvasGroup,
    },
    symbols::{get_field_from_name, get_field_object_value, get_method_addr, get_type_object_for_class},
    types::*,
};

static mut CLASS: *mut Il2CppClass = 0 as _;
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

static mut TYPE_OBJECT: *mut Il2CppObject = 0 as _;
pub fn type_object() -> *mut Il2CppObject {
    unsafe { TYPE_OBJECT }
}

static IN_PLAY_VOICE_INTERNAL: AtomicBool = AtomicBool::new(false);

pub fn is_in_play_voice_internal() -> bool {
    IN_PLAY_VOICE_INTERNAL.load(Ordering::Acquire)
}

struct InPlayVoiceGuard;
impl InPlayVoiceGuard {
    fn new() -> Self {
        IN_PLAY_VOICE_INTERNAL.store(true, Ordering::Release);
        Self
    }
}
impl Drop for InPlayVoiceGuard {
    fn drop(&mut self) {
        IN_PLAY_VOICE_INTERNAL.store(false, Ordering::Release);
    }
}

static mut GET_ISPLAYING_ADDR: usize = 0;
pub fn get_IsPlaying(this: *mut Il2CppObject) -> bool {
    let addr = unsafe { GET_ISPLAYING_ADDR };
    if addr == 0 || this.is_null() { return false; }
    let orig_fn: extern "C" fn(*mut Il2CppObject) -> bool =
        unsafe { std::mem::transmute(addr) };
    orig_fn(this)
}

static mut IS_OPEN_ADDR: usize = 0;
pub fn IsOpen(this: *mut Il2CppObject) -> bool {
    let addr = unsafe { IS_OPEN_ADDR };
    if addr == 0 || this.is_null() { return false; }
    let orig_fn: extern "C" fn(*mut Il2CppObject) -> bool =
        unsafe { std::mem::transmute(addr) };
    orig_fn(this)
}

static mut MESSAGE_TEXT_FIELD: *mut FieldInfo = 0 as _;
static mut CANVAS_GROUP_FIELD: *mut FieldInfo = 0 as _;

pub fn get_message_text(this: *mut Il2CppObject) -> *mut Il2CppObject {
    if this.is_null() { return null_mut(); }
    get_field_object_value(this, unsafe { MESSAGE_TEXT_FIELD })
}

pub fn get_canvas_group(this: *mut Il2CppObject) -> *mut Il2CppObject {
    if this.is_null() { return null_mut(); }
    get_field_object_value(this, unsafe { CANVAS_GROUP_FIELD })
}

pub fn is_active_or_playing(this: *mut Il2CppObject) -> bool {
    if this.is_null() { return false; }
    if is_in_play_voice_internal() {
        return true;
    }
    if IsOpen(this) || get_IsPlaying(this) {
        return true;
    }
    let cg = get_canvas_group(this);
    if !cg.is_null() && CanvasGroup::get_alpha(cg) > 0.01 {
        return true;
    }
    let mt = get_message_text(this);
    if !mt.is_null() {
        let go = Component::get_gameObject(mt);
        if !go.is_null() && GameObject::get_activeSelf(go) {
            let text = Text::get_text(mt);
            if !text.is_null() && unsafe { (*text).length } > 0 {
                return true;
            }
        }
    }
    false
}

type PlayVoiceInternalFn = extern "C" fn(this: *mut Il2CppObject, system_text: *mut Il2CppObject, use_smooth_face_blend: bool);
extern "C" fn PlayVoiceInternal(this: *mut Il2CppObject, system_text: *mut Il2CppObject, use_smooth_face_blend: bool) {
    let _guard = InPlayVoiceGuard::new();
    get_orig_fn!(PlayVoiceInternal, PlayVoiceInternalFn)(this, system_text, use_smooth_face_blend);
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, PartsCharaMessageBase);

    unsafe {
        CLASS = PartsCharaMessageBase;
        TYPE_OBJECT = get_type_object_for_class(PartsCharaMessageBase);
        GET_ISPLAYING_ADDR = get_method_addr(PartsCharaMessageBase, c"get_IsPlaying", 0);
        IS_OPEN_ADDR = get_method_addr(PartsCharaMessageBase, c"IsOpen", 0);
        MESSAGE_TEXT_FIELD = get_field_from_name(PartsCharaMessageBase, c"_messageText");
        CANVAS_GROUP_FIELD = get_field_from_name(PartsCharaMessageBase, c"_canvasGroup");
    }

    let play_voice_internal_addr = get_method_addr(PartsCharaMessageBase, c"PlayVoiceInternal", 2);
    new_hook!(play_voice_internal_addr, PlayVoiceInternal);
}
