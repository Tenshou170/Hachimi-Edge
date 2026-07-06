use crate::il2cpp::{
    symbols::{get_field_from_name, get_field_object_value, set_field_object_value},
    types::*
};

static mut _POOL_FIELD: *mut FieldInfo = 0 as _;

pub fn get_pool(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_object_value(this, unsafe { _POOL_FIELD })
}

pub fn set_pool(this: *mut Il2CppObject, value: *mut Il2CppObject) {
    set_field_object_value(this, unsafe { _POOL_FIELD }, value)
}

pub fn init(Cute_Cri_Assembly: *const Il2CppImage) {
    get_class_or_return!(Cute_Cri_Assembly, "Cute.Cri", AudioControllerBase);

    unsafe {
        _POOL_FIELD = get_field_from_name(AudioControllerBase, c"pool");
    }
}
