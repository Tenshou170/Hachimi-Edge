use std::ptr::null_mut;

use crate::{
    core::{game::Region, Hachimi},
    il2cpp::{
        hook::{
            UnityEngine_CoreModule::{Component, GameObject, RectTransform, Transform},
            UnityEngine_TextRenderingModule::TextAnchor,
            UnityEngine_UI::Text,
        },
        symbols::{get_field_from_name, get_field_object_value, get_method_addr},
        types::*,
        utils::get_text_component,
    },
};

static mut STATUS_TEXT_BASE_OBJ_FIELD: *mut FieldInfo = null_mut();

fn apply_title_settings(text: *mut Il2CppObject) {
    if text.is_null() {
        return;
    }

    Text::set_alignment(text, TextAnchor::MiddleCenter);
    Text::set_resizeTextForBestFit(text, true);
    Text::set_resizeTextMaxSize(text, 30);

    let text_tr = Component::get_transform(text);
    if !text_tr.is_null() {
        RectTransform::set_sizeDelta(text_tr, Vector2_t { x: 120.0, y: 40.0 });

        let mut anchored_pos = RectTransform::get_anchoredPosition(text_tr);
        anchored_pos.x = -71.0;
        RectTransform::set_anchoredPosition(text_tr, anchored_pos);
    }
}

type InitFn = extern "C" fn(
    this: *mut Il2CppObject,
    race_type: i32,
    race_instance_id: i32,
    race_horse_datas: *mut Il2CppObject,
    view_info: *mut Il2CppObject,
    paddock_contents_holder: *mut Il2CppObject,
    running_style_button_callback: *mut Il2CppObject,
    is_force_disable_running_style_change: bool,
);

extern "C" fn Init(
    this: *mut Il2CppObject,
    race_type: i32,
    race_instance_id: i32,
    race_horse_datas: *mut Il2CppObject,
    view_info: *mut Il2CppObject,
    paddock_contents_holder: *mut Il2CppObject,
    running_style_button_callback: *mut Il2CppObject,
    is_force_disable_running_style_change: bool,
) {
    get_orig_fn!(Init, InitFn)(
        this,
        race_type,
        race_instance_id,
        race_horse_datas,
        view_info,
        paddock_contents_holder,
        running_style_button_callback,
        is_force_disable_running_style_change,
    );

    if this.is_null() {
        return;
    }

    let field = unsafe { STATUS_TEXT_BASE_OBJ_FIELD };
    if field.is_null() {
        return;
    }

    let status_text_base_obj: *mut Il2CppObject = get_field_object_value(this, field);
    if status_text_base_obj.is_null() {
        return;
    }

    let base_tr = GameObject::get_transform(status_text_base_obj);
    if base_tr.is_null() {
        return;
    }

    let child_count = Transform::get_childCount(base_tr);
    for i in 0..child_count {
        let child = Transform::GetChild(base_tr, i);
        if child.is_null() {
            continue;
        }
        let mut text = get_text_component(child, "Title");
        if text.is_null() {
            text = get_text_component(child, "Wiz");
        }
        apply_title_settings(text);
    }
}

pub fn init(umamusume: *const Il2CppImage) {
    if Hachimi::instance().game.region != Region::Japan {
        return;
    }

    get_class_or_return!(umamusume, Gallop, PaddockCharaStatus);

    unsafe {
        STATUS_TEXT_BASE_OBJ_FIELD = get_field_from_name(PaddockCharaStatus, c"_statusTextBaseObj");
    }

    let Init_addr = get_method_addr(PaddockCharaStatus, c"Init", 7);
    new_hook!(Init_addr, Init);
}
