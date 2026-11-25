use std::ptr::null_mut;

use crate::{
    core::{game::Region, Hachimi},
    il2cpp::{
        ext::StringExt,
        hook::{
            UnityEngine_CoreModule::{Component, GameObject, Transform},
            UnityEngine_UI::Text,
        },
        symbols::{get_field_from_name, get_field_object_value, get_method_addr, get_type_object_for_class},
        types::*,
    },
};

use super::{ProperIcon, TextCommon};

static mut CLASS: *mut Il2CppClass = null_mut();
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

static mut TYPE_OBJECT: *mut Il2CppObject = null_mut();
pub fn type_object() -> *mut Il2CppObject {
    unsafe { TYPE_OBJECT }
}

// PartsCharacterProperList backing fields
static mut PROPER_TURF_ICON_FIELD: *mut FieldInfo = null_mut();
pub fn get_ProperTurfIcon(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_object_value(this, unsafe { PROPER_TURF_ICON_FIELD })
}

static mut PROPER_DIRT_ICON_FIELD: *mut FieldInfo = null_mut();
pub fn get_ProperDirtIcon(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_object_value(this, unsafe { PROPER_DIRT_ICON_FIELD })
}

static mut PROPER_SHORT_ICON_FIELD: *mut FieldInfo = null_mut();
pub fn get_ProperShortIcon(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_object_value(this, unsafe { PROPER_SHORT_ICON_FIELD })
}

static mut PROPER_MILE_ICON_FIELD: *mut FieldInfo = null_mut();
pub fn get_ProperMileIcon(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_object_value(this, unsafe { PROPER_MILE_ICON_FIELD })
}

static mut PROPER_MIDDLE_ICON_FIELD: *mut FieldInfo = null_mut();
pub fn get_ProperMiddleIcon(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_object_value(this, unsafe { PROPER_MIDDLE_ICON_FIELD })
}

static mut PROPER_LONG_ICON_FIELD: *mut FieldInfo = null_mut();
pub fn get_ProperLongIcon(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_object_value(this, unsafe { PROPER_LONG_ICON_FIELD })
}

static mut PROPER_NIGE_ICON_FIELD: *mut FieldInfo = null_mut();
pub fn get_ProperNigeIcon(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_object_value(this, unsafe { PROPER_NIGE_ICON_FIELD })
}

static mut PROPER_SENKO_ICON_FIELD: *mut FieldInfo = null_mut();
pub fn get_ProperSenkoIcon(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_object_value(this, unsafe { PROPER_SENKO_ICON_FIELD })
}

static mut PROPER_SASHI_ICON_FIELD: *mut FieldInfo = null_mut();
pub fn get_ProperSashiIcon(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_object_value(this, unsafe { PROPER_SASHI_ICON_FIELD })
}

static mut PROPER_OIKOMI_ICON_FIELD: *mut FieldInfo = null_mut();
pub fn get_ProperOikomiIcon(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_object_value(this, unsafe { PROPER_OIKOMI_ICON_FIELD })
}

const ICON_GETTERS: [fn(*mut Il2CppObject) -> *mut Il2CppObject; 10] = [
    get_ProperTurfIcon,
    get_ProperDirtIcon,
    get_ProperShortIcon,
    get_ProperMileIcon,
    get_ProperMiddleIcon,
    get_ProperLongIcon,
    get_ProperNigeIcon,
    get_ProperSenkoIcon,
    get_ProperSashiIcon,
    get_ProperOikomiIcon,
];

const HEADER_PATHS: [&str; 3] = [
    "ContentsRoot/ProperGroundRoot/ProperGroundText",
    "ContentsRoot/ProperDistanceRoot/ProperDistanceText",
    "ContentsRoot/ProperRunStyleRoot/ProperRunStyleText",
];

fn get_text_component(tr: *mut Il2CppObject, path: &str) -> *mut Il2CppObject {
    let child = Transform::Find(tr, path.to_il2cpp_string());
    if child.is_null() {
        return null_mut();
    }
    let go = Component::get_gameObject(child);
    if go.is_null() {
        return null_mut();
    }
    let text = GameObject::GetComponent(go, TextCommon::type_object());
    if !text.is_null() {
        text
    } else {
        GameObject::GetComponent(go, Text::type_object())
    }
}

type SetupFn = extern "C" fn(this: *mut Il2CppObject, parameter: *mut Il2CppObject, display_type: i32);
extern "C" fn Setup(this: *mut Il2CppObject, parameter: *mut Il2CppObject, display_type: i32) {
    get_orig_fn!(Setup, SetupFn)(this, parameter, display_type);

    if this.is_null() {
        return;
    }

    let tr = Component::get_transform(this);
    if tr.is_null() {
        return;
    }

    // Resolve 3 header texts
    let mut headers = [null_mut(); 3];
    for (i, &path) in HEADER_PATHS.iter().enumerate() {
        let text = get_text_component(tr, path);
        if text.is_null() {
            warn!("[PartsCharacterProperList] Header text not found: {path}");
            return;
        }
        headers[i] = text;
    }

    // Resolve 10 icons and validate their title components
    let mut icons = [null_mut(); 10];
    for (i, &getter) in ICON_GETTERS.iter().enumerate() {
        let icon = getter(this);
        let text = ProperIcon::get_title(icon);
        let text_tr = Component::get_transform(text);
        if text.is_null() || text_tr.is_null() {
            warn!("[PartsCharacterProperList] Icon text at index {i} not found");
            return;
        }
        icons[i] = icon;
    }

    // Apply header adjustments
    for text in headers {
        Text::set_fontSize(text, 32);
        Text::set_horizontalOverflow(text, 1);
    }

    // Delegate icon title adjustments to ProperIcon
    for icon in icons {
        ProperIcon::apply_title_settings(icon);
    }
}

pub fn init(umamusume: *const Il2CppImage) {
    if Hachimi::instance().game.region != Region::Japan {
        return;
    }

    get_class_or_return!(umamusume, Gallop, PartsCharacterProperList);

    unsafe {
        CLASS = PartsCharacterProperList;
        TYPE_OBJECT = get_type_object_for_class(PartsCharacterProperList);

        PROPER_TURF_ICON_FIELD = get_field_from_name(PartsCharacterProperList, c"<ProperTurfIcon>k__BackingField");
        PROPER_DIRT_ICON_FIELD = get_field_from_name(PartsCharacterProperList, c"<ProperDirtIcon>k__BackingField");
        PROPER_SHORT_ICON_FIELD = get_field_from_name(PartsCharacterProperList, c"<ProperShortIcon>k__BackingField");
        PROPER_MILE_ICON_FIELD = get_field_from_name(PartsCharacterProperList, c"<ProperMileIcon>k__BackingField");
        PROPER_MIDDLE_ICON_FIELD = get_field_from_name(PartsCharacterProperList, c"<ProperMiddleIcon>k__BackingField");
        PROPER_LONG_ICON_FIELD = get_field_from_name(PartsCharacterProperList, c"<ProperLongIcon>k__BackingField");
        PROPER_NIGE_ICON_FIELD = get_field_from_name(PartsCharacterProperList, c"<ProperNigeIcon>k__BackingField");
        PROPER_SENKO_ICON_FIELD = get_field_from_name(PartsCharacterProperList, c"<ProperSenkoIcon>k__BackingField");
        PROPER_SASHI_ICON_FIELD = get_field_from_name(PartsCharacterProperList, c"<ProperSashiIcon>k__BackingField");
        PROPER_OIKOMI_ICON_FIELD = get_field_from_name(PartsCharacterProperList, c"<ProperOikomiIcon>k__BackingField");
    }

    let Setup_addr = get_method_addr(PartsCharacterProperList, c"Setup", 2);
    new_hook!(Setup_addr, Setup);
}
