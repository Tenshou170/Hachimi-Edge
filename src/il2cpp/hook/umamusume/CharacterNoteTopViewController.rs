use crate::{
    core::{hachimi::UITextConfig, Hachimi},
    il2cpp::{
        ext::StringExt,
        hook::{
            UnityEngine_CoreModule::{Component, GameObject, Object, RectTransform},
            UnityEngine_TextRenderingModule::TextAnchor,
            UnityEngine_UI::Text,
        },
        symbols::get_method_addr,
        types::*,
    },
};

use super::{ButtonCommon, CharacterNoteTopView, TextCommon, ViewControllerBase};
use std::collections::HashMap;
use std::sync::Mutex;
use once_cell::sync::Lazy;

#[derive(Clone, Copy)]
struct StoredPosition {
    base: Vector2_t,
    applied: Vector2_t,
}

static ORIGINAL_POSITIONS: Lazy<Mutex<HashMap<i32, StoredPosition>>> = Lazy::new(|| Mutex::new(HashMap::new()));

type InitializeViewFn = extern "C" fn(this: *mut Il2CppObject) -> *mut Il2CppObject;
extern "C" fn InitializeView(this: *mut Il2CppObject) -> *mut Il2CppObject {
    let view = ViewControllerBase::GetView(this);
    let config = &Hachimi::instance().localized_data.load().config;

    if let Some(config) = config.character_note_top_gallery_button.as_ref() {
        let gallery_button = CharacterNoteTopView::get_ButtonGallery(view);
        apply_gallery_button_config(gallery_button, config);
    }

    if let Some(config) = config.character_note_top_talk_gallery_button.as_ref() {
        let talk_gallery_button = CharacterNoteTopView::get_ButtonTalkGallery(view);
        apply_gallery_button_config(talk_gallery_button, config);
    }

    get_orig_fn!(InitializeView, InitializeViewFn)(this)
}

fn apply_gallery_button_config(button: *mut Il2CppObject, config: &UITextConfig) {
    if button.is_null() || !Object::op_Implicit(button) {
        return;
    }
    let target_text = ButtonCommon::get_TargetText(button);
    if target_text.is_null() || !Object::op_Implicit(target_text) {
        return;
    }

    if let Some(text) = config.text.as_ref() {
        let game_object = Component::get_gameObject(button);
        if !game_object.is_null() && Object::op_Implicit(game_object) {
            let text_objects = GameObject::GetComponentsInChildren(game_object, TextCommon::type_object(), true);

            if !text_objects.this.is_null() {
                let empty_str = "".to_il2cpp_string();
                for text_object in unsafe { text_objects.as_slice().iter() } {
                    if !text_object.is_null() && Object::op_Implicit(*text_object) && *text_object != target_text {
                        Text::set_text(*text_object, empty_str);
                    }
                }
            }
        }

        Text::set_horizontalOverflow(target_text, 1);
        Text::set_alignment(target_text, TextAnchor::UpperLeft);
        Text::set_text(target_text, text.to_il2cpp_string());
    }

    if let Some(font_size) = config.font_size {
        Text::set_fontSize(target_text, font_size);
    }

    if let Some(line_spacing) = config.line_spacing {
        Text::set_lineSpacing(target_text, line_spacing);
    }
    
    // Apply position offset
    if config.position_offset_x.is_some() || config.position_offset_y.is_some() {
        let rect_transform = Component::get_transform(target_text);
        if !rect_transform.is_null() && Object::op_Implicit(rect_transform) {
            let instance_id = Object::get_instanceID(rect_transform);
            let current_pos = RectTransform::get_anchoredPosition(rect_transform);

            let mut map = ORIGINAL_POSITIONS.lock().unwrap();
            let base_pos = match map.get(&instance_id) {
                Some(stored) => {
                    if (current_pos.x - stored.applied.x).abs() < 0.1 && (current_pos.y - stored.applied.y).abs() < 0.1 {
                        stored.base
                    } else {
                        current_pos
                    }
                }
                None => current_pos,
            };

            let target_x = config.position_offset_x.map(|ox| base_pos.x + ox).unwrap_or(base_pos.x);
            let target_y = config.position_offset_y.map(|oy| base_pos.y + oy).unwrap_or(base_pos.y);
            let target_pos = Vector2_t { x: target_x, y: target_y };

            map.insert(instance_id, StoredPosition {
                base: base_pos,
                applied: target_pos,
            });
            drop(map);

            if (current_pos.x - target_pos.x).abs() >= 0.01 || (current_pos.y - target_pos.y).abs() >= 0.01 {
                RectTransform::set_anchoredPosition(rect_transform, target_pos);
            }
        }
    }
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, CharacterNoteTopViewController);

    let InitializeView_addr = get_method_addr(CharacterNoteTopViewController, c"InitializeView", 0);

    new_hook!(InitializeView_addr, InitializeView);
}
