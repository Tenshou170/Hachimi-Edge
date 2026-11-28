use std::{ffi::CStr, ptr::null_mut};

use crate::{
    core::{game::Region, Hachimi},
    il2cpp::{
        api::il2cpp_class_get_name,
        ext::Il2CppObjectExt,
        hook::{
            UnityEngine_CoreModule::{Behaviour, Component, GameObject, RectTransform, Transform},
            UnityEngine_TextRenderingModule::TextAnchor,
            UnityEngine_UI::{ContentSizeFitter, Text},
        },
        symbols::{
            get_field_from_name, get_field_object_value, get_method_addr, get_type_object_for_class,
        },
        types::*,
    },
};

static mut POPULARITY_TEXT_FIELD: *mut FieldInfo = null_mut();
static mut RUN_STYLE_FIELD: *mut FieldInfo = null_mut();
static mut CHARA_NAME_FIELD: *mut FieldInfo = null_mut();
static mut USER_NAME_FIELD: *mut FieldInfo = null_mut();
static mut NICK_NAME_RIBBON_FIELD: *mut FieldInfo = null_mut();

static mut ROTATE_TEXT_TYPE_OBJECT: *mut Il2CppObject = null_mut();
static mut ROTATE_TEXT_SIZE_SYNC_TYPE_OBJECT: *mut Il2CppObject = null_mut();

fn apply_text_settings(text: *mut Il2CppObject) {
    if text.is_null() {
        return;
    }

    Text::set_resizeTextForBestFit(text, true);
    Text::set_horizontalOverflow(text, 0);

    let text_tr = Component::get_transform(text);
    if !text_tr.is_null() {
        RectTransform::set_sizeDelta(text_tr, Vector2_t { x: 150.0, y: 30.0 });
    }
}

fn disable_component(go: *mut Il2CppObject, type_obj: *mut Il2CppObject) {
    if go.is_null() || type_obj.is_null() {
        return;
    }
    let comp = GameObject::GetComponent(go, type_obj);
    if !comp.is_null() {
        Behaviour::set_enabled(comp, false);
    }
}

fn apply_landscape_overrides(this: *mut Il2CppObject) {
    let chara_name: *mut Il2CppObject = unsafe {
        if !CHARA_NAME_FIELD.is_null() {
            get_field_object_value(this, CHARA_NAME_FIELD)
        } else {
            null_mut()
        }
    };
    if !chara_name.is_null() {
        disable_component(Component::get_gameObject(chara_name), unsafe {
            ROTATE_TEXT_TYPE_OBJECT
        });
        Text::set_fontSize(chara_name, 30);
        Text::set_resizeTextForBestFit(chara_name, true);
        Text::set_resizeTextMaxSize(chara_name, 30);
    }

    let user_name: *mut Il2CppObject = unsafe {
        if !USER_NAME_FIELD.is_null() {
            get_field_object_value(this, USER_NAME_FIELD)
        } else {
            null_mut()
        }
    };
    if !user_name.is_null() {
        disable_component(Component::get_gameObject(user_name), unsafe {
            ROTATE_TEXT_TYPE_OBJECT
        });
    }

    let nick_ribbon: *mut Il2CppObject = unsafe {
        if !NICK_NAME_RIBBON_FIELD.is_null() {
            get_field_object_value(this, NICK_NAME_RIBBON_FIELD)
        } else {
            null_mut()
        }
    };
    if !nick_ribbon.is_null() {
        let ribbon_tr = Component::get_transform(nick_ribbon);
        if !ribbon_tr.is_null() {
            RectTransform::set_sizeDelta(ribbon_tr, Vector2_t { x: 44.0, y: 330.4 });
            RectTransform::set_anchoredPosition(ribbon_tr, Vector2_t { x: 100.0, y: -2.0 });

            if Transform::get_childCount(ribbon_tr) > 0 {
                let name_tr = Transform::GetChild(ribbon_tr, 0);
                if !name_tr.is_null() {
                    let name_go = Component::get_gameObject(name_tr);
                    if !name_go.is_null() {
                        disable_component(name_go, unsafe { ROTATE_TEXT_TYPE_OBJECT });
                        disable_component(name_go, unsafe { ROTATE_TEXT_SIZE_SYNC_TYPE_OBJECT });
                        disable_component(name_go, ContentSizeFitter::type_object());

                        let name_text = GameObject::GetComponent(name_go, Text::type_object());
                        if !name_text.is_null() {
                            Text::set_alignment(name_text, TextAnchor::MiddleCenter);
                            Text::set_fontSize(name_text, 24);
                            Text::set_resizeTextForBestFit(name_text, true);
                            Text::set_resizeTextMaxSize(name_text, 24);
                        }
                    }
                    RectTransform::set_sizeDelta(name_tr, Vector2_t { x: 310.0, y: 28.0 });
                    RectTransform::set_anchoredPosition(name_tr, Vector2_t { x: 0.0, y: 2.0 });
                }
            }
        }
    }
}

type SetupCommonFn = extern "C" fn(
    this: *mut Il2CppObject,
    is_visible_user_name: bool,
    data: *mut Il2CppObject,
    panel: *mut Il2CppObject,
);

extern "C" fn SetupCommon(
    this: *mut Il2CppObject,
    is_visible_user_name: bool,
    data: *mut Il2CppObject,
    panel: *mut Il2CppObject,
) {
    get_orig_fn!(SetupCommon, SetupCommonFn)(this, is_visible_user_name, data, panel);

    if this.is_null() {
        return;
    }

    let fields = unsafe { [POPULARITY_TEXT_FIELD, RUN_STYLE_FIELD] };
    for field in fields {
        if !field.is_null() {
            apply_text_settings(get_field_object_value(this, field));
        }
    }

    let is_landscape = if panel.is_null() {
        false
    } else {
        let class_name = unsafe { il2cpp_class_get_name((*panel).klass()) };
        !class_name.is_null()
            && unsafe { CStr::from_ptr(class_name).to_bytes() == b"RaceEntryTablePanelLandscape" }
    };
    if is_landscape
        && Hachimi::instance()
            .localized_data
            .load()
            .config
            .race_entry_landscape_disable_tategaki
    {
        apply_landscape_overrides(this);
    }
}

pub fn init(umamusume: *const Il2CppImage) {
    if Hachimi::instance().game.region != Region::Japan {
        return;
    }

    get_class_or_return!(umamusume, Gallop, RaceEntryTableButton);
    get_class_or_return!(umamusume, Gallop, RotateText);
    get_class_or_return!(umamusume, Gallop, RotateTextSizeSync);

    unsafe {
        POPULARITY_TEXT_FIELD = get_field_from_name(RaceEntryTableButton, c"_popularityText");
        RUN_STYLE_FIELD = get_field_from_name(RaceEntryTableButton, c"_runStyle");
        CHARA_NAME_FIELD = get_field_from_name(RaceEntryTableButton, c"_charaName");
        USER_NAME_FIELD = get_field_from_name(RaceEntryTableButton, c"_userName");
        NICK_NAME_RIBBON_FIELD = get_field_from_name(RaceEntryTableButton, c"_nickNameRibbon");

        ROTATE_TEXT_TYPE_OBJECT = get_type_object_for_class(RotateText);
        ROTATE_TEXT_SIZE_SYNC_TYPE_OBJECT = get_type_object_for_class(RotateTextSizeSync);
    }

    let SetupCommon_addr = get_method_addr(RaceEntryTableButton, c"SetupCommon", 3);
    new_hook!(SetupCommon_addr, SetupCommon);
}
