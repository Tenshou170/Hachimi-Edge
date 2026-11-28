use std::ptr::null_mut;

use crate::{
    core::{game::Region, Hachimi},
    il2cpp::{
        hook::{
            UnityEngine_CoreModule::{Component, RectTransform},
            UnityEngine_UI::Text,
        },
        symbols::{get_field_from_name, get_field_object_value, get_method_addr},
        types::*,
        utils::get_text_component,
    },
};

static mut PROPER_TURF_TEXT_FIELD: *mut FieldInfo = null_mut();
static mut PROPER_DIRT_TEXT_FIELD: *mut FieldInfo = null_mut();
static mut PROPER_SHORT_TEXT_FIELD: *mut FieldInfo = null_mut();
static mut PROPER_MILE_TEXT_FIELD: *mut FieldInfo = null_mut();
static mut PROPER_MIDDLE_TEXT_FIELD: *mut FieldInfo = null_mut();
static mut PROPER_LONG_TEXT_FIELD: *mut FieldInfo = null_mut();
static mut STYLE_ROOT_FIELD: *mut FieldInfo = null_mut();

const STYLE_LABELS: [&str; 4] = ["逃げ", "先行", "差し", "追込"];

fn apply_text_settings(text: *mut Il2CppObject) {
    if text.is_null() {
        return;
    }

    Text::set_resizeTextForBestFit(text, true);
    Text::set_resizeTextMaxSize(text, 32);

    let text_tr = Component::get_transform(text);
    if !text_tr.is_null() {
        RectTransform::set_sizeDelta(text_tr, Vector2_t { x: 114.0, y: 39.0 });
    }
}

fn on_setup(this: *mut Il2CppObject) {
    if this.is_null() {
        return;
    }

    let fields = unsafe {
        [
            PROPER_TURF_TEXT_FIELD,
            PROPER_DIRT_TEXT_FIELD,
            PROPER_SHORT_TEXT_FIELD,
            PROPER_MILE_TEXT_FIELD,
            PROPER_MIDDLE_TEXT_FIELD,
            PROPER_LONG_TEXT_FIELD,
        ]
    };

    for field in fields {
        if !field.is_null() {
            apply_text_settings(get_field_object_value(this, field));
        }
    }

    let style_root_field = unsafe { STYLE_ROOT_FIELD };
    if !style_root_field.is_null() {
        let style_root: *mut Il2CppObject = get_field_object_value(this, style_root_field);
        if !style_root.is_null() {
            for label in STYLE_LABELS {
                apply_text_settings(get_text_component(style_root, label));
            }
        }
    }
}

type Setup3Fn = extern "C" fn(
    this: *mut Il2CppObject,
    trained_data: *mut Il2CppObject,
    disp_type: i32,
    course_info: *mut Il2CppObject,
);

extern "C" fn Setup3(
    this: *mut Il2CppObject,
    trained_data: *mut Il2CppObject,
    disp_type: i32,
    course_info: *mut Il2CppObject,
) {
    get_orig_fn!(Setup3, Setup3Fn)(this, trained_data, disp_type, course_info);
    on_setup(this);
}

pub fn init(umamusume: *const Il2CppImage) {
    if Hachimi::instance().game.region != Region::Japan {
        return;
    }

    get_class_or_return!(umamusume, Gallop, PartsSingleModeStartProperStatus);

    unsafe {
        PROPER_TURF_TEXT_FIELD =
            get_field_from_name(PartsSingleModeStartProperStatus, c"_properTurfText");
        PROPER_DIRT_TEXT_FIELD =
            get_field_from_name(PartsSingleModeStartProperStatus, c"_properDirtText");
        PROPER_SHORT_TEXT_FIELD =
            get_field_from_name(PartsSingleModeStartProperStatus, c"_properShortText");
        PROPER_MILE_TEXT_FIELD =
            get_field_from_name(PartsSingleModeStartProperStatus, c"_properMileText");
        PROPER_MIDDLE_TEXT_FIELD =
            get_field_from_name(PartsSingleModeStartProperStatus, c"_properMiddleText");
        PROPER_LONG_TEXT_FIELD =
            get_field_from_name(PartsSingleModeStartProperStatus, c"_properLongText");
        STYLE_ROOT_FIELD = get_field_from_name(PartsSingleModeStartProperStatus, c"_styleRoot");
    }

    let Setup3_addr = get_method_addr(PartsSingleModeStartProperStatus, c"Setup", 3);
    new_hook!(Setup3_addr, Setup3);
}
