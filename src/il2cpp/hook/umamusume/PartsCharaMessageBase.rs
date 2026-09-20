use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

use crate::il2cpp::{
    symbols::{get_method_addr, get_type_object_for_class},
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

// ── PlayVoiceInternal guard ───────────────────────────────────────────────────────────────
// When PlayVoiceInternal is executing, a speech bubble is actively initiating a voice line.
// We can suppress captions immediately at that point without waiting for Open() to fire.
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

// ── Open / Close bubble counter ───────────────────────────────────────────────────────────
// Hooks PartsCharaMessageBase::Open() (Slot 13) and Close() (Slot 14) — both virtual, so
// all 26+ subclasses (PartsHomeCharaMessage, PartsDailyRaceTopCharaMessage, etc.) are
// covered by a single pair of hooks on the base class.
//
// The counter is used by AudioManager::has_active_speech_bubble() instead of the old
// Object::FindObjectsOfType() scan, making the check O(1) and reliable across all views.
static ACTIVE_BUBBLE_COUNT: AtomicI32 = AtomicI32::new(0);

pub fn active_bubble_count() -> i32 {
    ACTIVE_BUBBLE_COUNT.load(Ordering::Acquire)
}

type PartsCharaMessageBase_OpenFn = extern "C" fn(this: *mut Il2CppObject);
extern "C" fn PartsCharaMessageBase_Open(this: *mut Il2CppObject) {
    ACTIVE_BUBBLE_COUNT.fetch_add(1, Ordering::AcqRel);
    get_orig_fn!(PartsCharaMessageBase_Open, PartsCharaMessageBase_OpenFn)(this);
}

type PartsCharaMessageBase_CloseFn = extern "C" fn(this: *mut Il2CppObject);
extern "C" fn PartsCharaMessageBase_Close(this: *mut Il2CppObject) {
    // Saturate at 0 — if Close fires without a matching Open (e.g. on first scene load)
    // we must not go negative.
    ACTIVE_BUBBLE_COUNT.fetch_update(Ordering::AcqRel, Ordering::Acquire, |v| {
        Some(if v > 0 { v - 1 } else { 0 })
    }).ok();
    get_orig_fn!(PartsCharaMessageBase_Close, PartsCharaMessageBase_CloseFn)(this);
}

// ── PlayVoiceInternal hook ────────────────────────────────────────────────────────────────
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
    }

    let play_voice_internal_addr = get_method_addr(PartsCharaMessageBase, c"PlayVoiceInternal", 2);
    new_hook!(play_voice_internal_addr, PlayVoiceInternal);

    // Hook the virtual Open/Close pair on the base class — covers all subclasses.
    let open_addr = get_method_addr(PartsCharaMessageBase, c"Open", 0);
    new_hook!(open_addr, PartsCharaMessageBase_Open);

    let close_addr = get_method_addr(PartsCharaMessageBase, c"Close", 0);
    new_hook!(close_addr, PartsCharaMessageBase_Close);
}
