use crate::{
    core::live_utils::AudioPlayback,
    il2cpp::{
        symbols::{get_method_addr, get_field_from_name, get_field_object_value, set_field_object_value,
                  get_field_value, set_field_value},
        types::*
    }
};

static mut CLASS: *mut Il2CppClass = 0 as _;

pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

static mut _SOURCELIST_FIELD: *mut FieldInfo = 0 as _;

pub fn get_sourceList(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_object_value(this, unsafe { _SOURCELIST_FIELD })
}

pub fn set_sourceList(this: *mut Il2CppObject, value: *mut Il2CppObject) {
    set_field_object_value(this, unsafe { _SOURCELIST_FIELD }, value)
}

static mut _USINGINDEX_FIELD: *mut FieldInfo = 0 as _;

pub fn get_usingIndex(this: *mut Il2CppObject) -> i32 {
    get_field_value(this, unsafe { _USINGINDEX_FIELD })
}

pub fn set_usingIndex(this: *mut Il2CppObject, value: i32) {
    set_field_value(this, unsafe { _USINGINDEX_FIELD }, &value)
}

static mut IS_SAME_PLAYBACK_ID_ADDR: usize = 0;

pub fn IsSamePlaybackId(this: *mut Il2CppObject, playback: AudioPlayback) -> bool {
    let addr = unsafe { IS_SAME_PLAYBACK_ID_ADDR };
    if addr == 0 { return false; }
    let orig_fn: extern "C" fn(*mut Il2CppObject, AudioPlayback) -> bool =
        unsafe { std::mem::transmute(addr) };
    orig_fn(this, playback)
}

pub fn init(Cute_Cri_Assembly: *const Il2CppImage) {
    get_class_or_return!(Cute_Cri_Assembly, "Cute.Cri", CuteAudioSource);

    unsafe {
        CLASS = CuteAudioSource;
        _SOURCELIST_FIELD  = get_field_from_name(CuteAudioSource, c"sourceList");
        _USINGINDEX_FIELD  = get_field_from_name(CuteAudioSource, c"usingIndex");
        IS_SAME_PLAYBACK_ID_ADDR = get_method_addr(CuteAudioSource, c"IsSamePlaybackId", 1);
    }
}
