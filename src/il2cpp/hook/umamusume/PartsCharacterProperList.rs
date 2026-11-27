use std::ptr::null_mut;

use crate::{
    core::{game::Region, Hachimi},
    il2cpp::{
        ext::Il2CppObjectExt,
        hook::{
            UnityEngine_CoreModule::{Component, RectTransform, Transform},
            UnityEngine_UI::Text,
        },
        symbols::{get_field_from_name, get_field_object_value, get_method_addr},
        types::*,
        utils::get_text_component,
    },
};

use super::ProperIcon;

static mut CLASS: *mut Il2CppClass = null_mut();

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

const LOG_RESULTS_LAYOUTS: [(&[fn(*mut Il2CppObject) -> *mut Il2CppObject], f32, f32); 2] = [
    (&[get_ProperTurfIcon, get_ProperShortIcon, get_ProperNigeIcon], -34.0, -42.0),
    (&[get_ProperDirtIcon, get_ProperMileIcon, get_ProperMiddleIcon, get_ProperLongIcon, get_ProperSenkoIcon, get_ProperSashiIcon, get_ProperOikomiIcon], -40.0, -22.0),
];

fn is_log_results(tr: *mut Il2CppObject) -> bool {
    if unsafe { &*tr }.name() != "PartsSingleModeProperList" {
        return false;
    }

    let mut curr = Transform::get_parent(tr);
    for _ in 0..16 {
        if curr.is_null() {
            break;
        }
        if unsafe { &*curr }.name().starts_with("PartsIdleSingleModeResultLogTopics") {
            return true;
        }
        curr = Transform::get_parent(curr);
    }
    false
}

const HEADER_PATHS: [&str; 3] = [
    "ContentsRoot/ProperGroundRoot/ProperGroundText",
    "ContentsRoot/ProperDistanceRoot/ProperDistanceText",
    "ContentsRoot/ProperRunStyleRoot/ProperRunStyleText",
];

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

    // apply header adjustments
    for &path in &HEADER_PATHS {
        let text = get_text_component(tr, path);
        if text.is_null() {
            warn!("[PartsCharacterProperList] Header text not found: {path}");
            continue;
        }
        Text::set_resizeTextForBestFit(text, true);
        Text::set_resizeTextMaxSize(text, 34);

        let text_tr = Component::get_transform(text);
        if !text_tr.is_null() {
            let mut size_delta = RectTransform::get_sizeDelta(text_tr);
            size_delta.x = 184.93289;
            RectTransform::set_sizeDelta(text_tr, size_delta);

            let mut anchored_pos = RectTransform::get_anchoredPosition(text_tr);
            anchored_pos.x = 96.7;
            RectTransform::set_anchoredPosition(text_tr, anchored_pos);
        }
    }

    // delegate adjustments to ProperIcon
    if is_log_results(tr) {
        for &(getters, anchored_pos_x, size_delta_x) in &LOG_RESULTS_LAYOUTS {
            for &getter in getters {
                ProperIcon::apply_title_layout(getter(this), anchored_pos_x, size_delta_x);
            }
        }
    } else {
        for &(getters, _, _) in &LOG_RESULTS_LAYOUTS {
            for &getter in getters {
                ProperIcon::apply_title_settings(getter(this));
            }
        }
    }
}

pub fn init(umamusume: *const Il2CppImage) {
    if Hachimi::instance().game.region != Region::Japan {
        return;
    }

    get_class_or_return!(umamusume, Gallop, PartsCharacterProperList);

    unsafe {
        CLASS = PartsCharacterProperList;

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
