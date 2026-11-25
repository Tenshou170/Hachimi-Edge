use std::ptr::null_mut;

use crate::{
    core::{game::Region, Hachimi},
    il2cpp::{
        hook::{
            UnityEngine_CoreModule::{Component, RectTransform},
            UnityEngine_UI::Text,
        },
        symbols::{get_field_from_name, get_field_object_value, get_type_object_for_class},
        types::*,
    },
};

static mut CLASS: *mut Il2CppClass = null_mut();
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

static mut TYPE_OBJECT: *mut Il2CppObject = null_mut();
pub fn type_object() -> *mut Il2CppObject {
    unsafe { TYPE_OBJECT }
}

static mut TITLE_FIELD: *mut FieldInfo = null_mut();
pub fn get_title(this: *mut Il2CppObject) -> *mut Il2CppObject {
    if this.is_null() {
        return null_mut();
    }
    get_field_object_value(this, unsafe { TITLE_FIELD })
}

pub fn apply_title_settings(this: *mut Il2CppObject) {
    let text = get_title(this);
    let text_tr = Component::get_transform(text);
    if text.is_null() || text_tr.is_null() {
        return;
    }

    Text::set_resizeTextForBestFit(text, true);
    Text::set_resizeTextMaxSize(text, 30);
    RectTransform::set_anchoredPosition(text_tr, Vector2_t { x: -21.21, y: 1.0 });
    RectTransform::set_sizeDelta(text_tr, Vector2_t { x: -61.61448, y: -9.05019 });
}

pub fn init(umamusume: *const Il2CppImage) {
    if Hachimi::instance().game.region != Region::Japan {
        return;
    }

    get_class_or_return!(umamusume, Gallop, ProperIcon);

    unsafe {
        CLASS = ProperIcon;
        TYPE_OBJECT = get_type_object_for_class(ProperIcon);
        TITLE_FIELD = get_field_from_name(ProperIcon, c"_title");
    }
}
