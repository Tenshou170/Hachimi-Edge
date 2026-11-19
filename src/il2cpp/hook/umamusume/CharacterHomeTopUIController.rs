use crate::{
    core::{hachimi::UITextConfig, Hachimi},
    il2cpp::{
        ext::{Il2CppStringExt, StringExt},
        symbols::get_method_addr,
        types::*,
        hook::{
            UnityEngine_CoreModule::{Component, GameObject, Object, RectTransform},
            UnityEngine_UI::Text as UIText,
        },
        api::{il2cpp_class_get_type, il2cpp_type_get_object},
    },
};

use super::{ButtonCommon, CharacterHomeTopUI, TextCommon};
use std::{collections::HashMap, sync::Mutex};
use once_cell::sync::Lazy;

#[derive(Clone, Copy)]
struct StoredPosition {
    base: Vector2_t,
    applied: Vector2_t,
}

static ORIGINAL_POSITIONS: Lazy<Mutex<HashMap<i32, StoredPosition>>> = Lazy::new(|| Mutex::new(HashMap::new()));

type UpdateViewFn = extern "C" fn(this: *mut Il2CppObject);

// enhance menu hook
extern "C" fn UpdateView(this: *mut Il2CppObject) {
    if this.is_null() {
        get_orig_fn!(UpdateView, UpdateViewFn)(this);
        return;
    }
    
    // validate context before proceeding
    if !is_valid_enhancement_context(this) {
        get_orig_fn!(UpdateView, UpdateViewFn)(this);
        return;
    }
    
    // call the UI first
    get_orig_fn!(UpdateView, UpdateViewFn)(this);
    
    // apply configs for enhance menu after calling UI
    apply_button_configs(this);
}

fn is_valid_enhancement_context(this: *mut Il2CppObject) -> bool {
    if this.is_null() || !Object::op_Implicit(this) {
        return false;
    }

    let card_button = CharacterHomeTopUI::get_cardRootButton(this);
    // op_Implicit also rejects destroyed Unity objects, which field getters can
    // still hand back during scene teardown (e.g. returning home from the
    // mission dialog while an asset unload is in flight).
    if card_button.is_null() || !Object::op_Implicit(card_button) {
        return false;
    }
    
    let game_object = Component::get_gameObject(card_button);
    if game_object.is_null() || !Object::op_Implicit(game_object) {
        return false;
    }
    
    let button_name = Object::get_name(game_object);
    if button_name.is_null() {
        return false;
    }
    
    let button_name_str = unsafe { (*button_name).as_utf16str().to_string() };
    let button_name_lower = button_name_str.to_lowercase();
    
    // skip unwanted contexts
    const EXCLUDED_CONTEXTS: &[&str] = &[
        "story", "race", "circle", "shop", 
        "gacha", "mission", "present"
    ];
    
    for &context in EXCLUDED_CONTEXTS {
        if button_name_lower.contains(context) {
            return false;
        }
    }
    
    true
}

fn apply_button_configs(this: *mut Il2CppObject) {
    let config = &Hachimi::instance().localized_data.load().config;
    let Some(overrides) = config.buttons_override.as_ref() else { return };

    macro_rules! apply { ($cfg:ident, $getter:ident) => {
        if let Some(c) = overrides.$cfg.as_ref() {
            let b = CharacterHomeTopUI::$getter(this);
            if !b.is_null() && Object::op_Implicit(b) { 
                apply_button_config(b, c, stringify!($cfg)); 
            }
        }
    }}

    apply!(character_home_top_card_root_button, get_cardRootButton);
    apply!(character_home_top_support_card_root_button, get_supportCardRootButton);
    apply!(character_home_top_trained_chara_root_button, get_trainedCharaRootButton);
    apply!(character_home_top_character_card_catalog_button, get_characterCardCatalogButton);
    apply!(character_home_top_card_lv_up_button, get_cardLvUpButton);
    apply!(character_home_top_hint_lv_up_button, get_hintLvUpButton);
    apply!(character_home_top_card_limit_break_button, get_cardLimitBreakButton);
    apply!(character_home_top_piece_exchange_button, get_pieceExchangeButton);
    apply!(character_home_top_support_edit_button, get_supportEditButton);
    apply!(character_home_top_support_sell_button, get_supportSellButton);
    apply!(character_home_top_support_list_button, get_supportListButton);
    apply!(character_home_top_trained_list_button, get_trainedListButton);
    apply!(character_home_top_support_lv_up_button, get_supportLvUpButton);
    apply!(character_home_top_support_limit_break_button, get_supportLimitBreakButton);
    
    // handle layout differences safely: new team edit button (JP) vs legacy team edit button (GLB/TW)
    if let Some(c) = overrides.character_home_top_new_team_edit_button.as_ref() {
        let b = CharacterHomeTopUI::get_newTeamEditButton(this);
        if !b.is_null() && Object::op_Implicit(b) {
            apply_button_config(b, c, "character_home_top_new_team_edit_button");
        } else {
            let b = CharacterHomeTopUI::get_teamEditButton(this);
            if !b.is_null() && Object::op_Implicit(b) {
                apply_button_config(b, c, "character_home_top_new_team_edit_button");
            }
        }
    }
    
    apply!(character_home_top_transfer_button, get_transferButton);
    apply!(character_home_top_trained_chara_root_short_button, get_trainedCharaRootShortButton);
    apply!(character_home_top_succession_only_chara_root_button, get_successionOnlyCharaRootButton);
    apply!(character_home_top_succession_only_start_button, get_successionOnlyStartButton);
    apply!(character_home_top_succession_only_list_button, get_successionOnlyListButton);
}

fn apply_button_config(button: *mut Il2CppObject, config: &UITextConfig, config_name: &str) {
    // Skip buttons whose underlying object was destroyed mid-frame (scene
    // teardown); touching them crashes inside the engine.
    if button.is_null() || !Object::op_Implicit(button) {
        return;
    }

    let text_components = collect_text_components(button, config_name);
    if text_components.is_empty() {
        return;
    }

    for (index, &text_component) in text_components.iter().enumerate() {
        if text_component.is_null() { continue; }
        apply_text_config(text_component, config, index);
        apply_position_offset(text_component, config, index);
    }
}

fn collect_text_components(button: *mut Il2CppObject, config_name: &str) -> Vec<*mut Il2CppObject> {
    let game_object = Component::get_gameObject(button);
    if game_object.is_null() {
        return Vec::new();
    }

    // we need to find some strings in some problematic buttons
    if config_name == "character_home_top_succession_only_start_button" {
        let text_class = UIText::class();
        if text_class.is_null() {
            return Vec::new();
        }
        let text_type_obj = il2cpp_type_get_object(il2cpp_class_get_type(text_class));
        if text_type_obj.is_null() {
            return Vec::new();
        }

        let mut custom_components = vec![std::ptr::null_mut(); 2];
        let text_objects = GameObject::GetComponentsInChildren(game_object, text_type_obj, true);
        if !text_objects.this.is_null() {
            let text_slice = unsafe { text_objects.as_slice() };
            for text_obj in text_slice {
                if text_obj.is_null() || !Object::op_Implicit(*text_obj) { continue; }
                let t_go = Component::get_gameObject(*text_obj);
                if t_go.is_null() { continue; }
                let name_ptr = Object::get_name(t_go);
                if name_ptr.is_null() { continue; }
                let name = unsafe { (*name_ptr).as_utf16str().to_string() };
                if name == "まとめて獲得" {
                    custom_components[0] = *text_obj; // text
                } else if name == "継承専用ウマ娘" {
                    custom_components[1] = *text_obj; // text2
                }
            }
        }
        return custom_components;
    }

    let mut text_components = Vec::new();

    let target_text = ButtonCommon::get_TargetText(button);
    if !target_text.is_null() && Object::op_Implicit(target_text) {
        text_components.push(target_text);
    }
    
    let text_objects = GameObject::GetComponentsInChildren(game_object, TextCommon::type_object(), true);
    if text_objects.this.is_null() {
        return text_components;
    }
    
    let text_slice = unsafe { text_objects.as_slice() };
    for text_obj in text_slice.iter() {
        if !text_obj.is_null() && Object::op_Implicit(*text_obj) && !text_components.contains(text_obj) {
            text_components.push(*text_obj);
        }
    }
    
    text_components
}

fn apply_text_config(text_component: *mut Il2CppObject, config: &UITextConfig, index: usize) {
    if text_component.is_null() || !Object::op_Implicit(text_component) {
        return;
    }

    // index 0 and rest is 'text', index 1 is 'text2'
    let text_to_apply = if index == 1 {
        config.text2.as_ref()
    } else {
        config.text.as_ref()
    };

    if let Some(text) = text_to_apply {
        UIText::set_text(text_component, text.to_il2cpp_string());
    }

    if let Some(font_size) = config.font_size {
        UIText::set_fontSize(text_component, font_size);
    }

    if let Some(line_spacing) = config.line_spacing {
        UIText::set_lineSpacing(text_component, line_spacing);
    }
}

fn apply_position_offset(text_component: *mut Il2CppObject, config: &UITextConfig, index: usize) {
    let offset_x = if index == 1 { config.position_offset_x2 } else { config.position_offset_x };
    let offset_y = if index == 1 { config.position_offset_y2 } else { config.position_offset_y };

    if offset_x.is_none() && offset_y.is_none() {
        return;
    }

    let Some(rect_transform) = get_rect_transform(text_component) else { return };
    if rect_transform.is_null() || !Object::op_Implicit(rect_transform) { return; }

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

    let target_x = offset_x.map(|ox| base_pos.x + ox).unwrap_or(base_pos.x);
    let target_y = offset_y.map(|oy| base_pos.y + oy).unwrap_or(base_pos.y);
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

fn get_rect_transform(text_component: *mut Il2CppObject) -> Option<*mut Il2CppObject> {
    if text_component.is_null() || !Object::op_Implicit(text_component) {
        return None;
    }

    let rect_transform = Component::get_transform(text_component);
    if rect_transform.is_null() || !Object::op_Implicit(rect_transform) {
        None
    } else {
        Some(rect_transform)
    }
}

pub fn init(Plugins: *const Il2CppImage) {
    get_class_or_return!(Plugins, Gallop, CharacterHomeTopUI);

    let UpdateView_addr = get_method_addr(CharacterHomeTopUI, c"UpdateView", 0);
    new_hook!(UpdateView_addr, UpdateView);
}
