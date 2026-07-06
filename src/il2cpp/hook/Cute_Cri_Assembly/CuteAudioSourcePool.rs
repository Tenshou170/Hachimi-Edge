use crate::il2cpp::{
    symbols::{get_field_from_name, get_field_object_value, set_field_object_value},
    types::*
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

pub fn init(Cute_Cri_Assembly: *const Il2CppImage) {
    get_class_or_return!(Cute_Cri_Assembly, "Cute.Cri", CuteAudioSourcePool);

    unsafe {
        CLASS = CuteAudioSourcePool;
        _SOURCELIST_FIELD = get_field_from_name(CuteAudioSourcePool, c"sourceList");
    }
}
