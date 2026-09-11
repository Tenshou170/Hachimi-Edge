use std::ptr::null_mut;

use crate::{
    core::{template, Hachimi},
    il2cpp::{
        ext::{Il2CppStringExt, StringExt}, 
        hook::UnityEngine_TextRenderingModule::{TextAnchor, TextGenerator::IgnoreTGFiltersContext}, 
        symbols::{get_field_from_name, get_field_object_value, get_field_value, get_method_addr, set_field_object_value, set_field_value}, 
        types::*
    }
};

// AnText layout state accessors (ported from kairusds/Hachimi-Edge)
static mut TEXTANCHOR_FIELD: *mut FieldInfo = null_mut();
pub fn get_textAnchor(this: *mut Il2CppObject) -> i32 {
    get_field_value(this, unsafe { TEXTANCHOR_FIELD })
}

static mut USEFIT_FIELD: *mut FieldInfo = null_mut();
pub fn get_useFit(this: *mut Il2CppObject) -> bool {
    get_field_value(this, unsafe { USEFIT_FIELD })
}
pub fn set_useFit(this: *mut Il2CppObject, value: bool) {
    set_field_value(this, unsafe { USEFIT_FIELD }, &value);
}

static mut USEWRAP_FIELD: *mut FieldInfo = null_mut();
pub fn get_useWrap(this: *mut Il2CppObject) -> bool {
    get_field_value(this, unsafe { USEWRAP_FIELD })
}
pub fn set_useWrap(this: *mut Il2CppObject, value: bool) {
    set_field_value(this, unsafe { USEWRAP_FIELD }, &value);
}

pub fn get_lineSpace(this: *mut Il2CppObject) -> f32 {
    get_field_value(this, unsafe { LINESPACE_FIELD })
}

pub fn set_lineSpace(this: *mut Il2CppObject, value: f32) {
    set_field_value(this, unsafe { LINESPACE_FIELD }, &value);
}

pub fn get_fontSize(this: *mut Il2CppObject) -> i32 {
    get_field_value(this, unsafe { FONTSIZE_FIELD })
}

pub fn set_fontSize(this: *mut Il2CppObject, value: i32) {
    set_field_value(this, unsafe { FONTSIZE_FIELD }, &value);
}

pub fn set_textAnchor(this: *mut Il2CppObject, value: i32) {
    set_field_value(this, unsafe { TEXTANCHOR_FIELD }, &value);
}

static mut SET_TEXT_FIT_ADDR: usize = 0;
impl_addr_wrapper_fn!(SetTextFit, SET_TEXT_FIT_ADDR, (), this: *mut Il2CppObject, enable: bool);

static mut SET_TEXT_WRAP_ADDR: usize = 0;
impl_addr_wrapper_fn!(SetTextWrap, SET_TEXT_WRAP_ADDR, (), this: *mut Il2CppObject, enable: bool);

static mut SET_TEXT_ANCHOR_ADDR: usize = 0;
impl_addr_wrapper_fn!(SetTextAnchor, SET_TEXT_ANCHOR_ADDR, (), this: *mut Il2CppObject, anchor: TextAnchor);

static mut SET_TEXT_LINESPACE_ADDR: usize = 0;
impl_addr_wrapper_fn!(SetTextLinespace, SET_TEXT_LINESPACE_ADDR, (), this: *mut Il2CppObject, lineSpace: f32);

static mut SET_TEXT_FONTSIZE_ADDR: usize = 0;
impl_addr_wrapper_fn!(SetTextFontSize, SET_TEXT_FONTSIZE_ADDR, (), this: *mut Il2CppObject, fontSize: i32);

// optimized out in assembly
static mut LINESPACE_FIELD: *mut FieldInfo = null_mut();
pub fn set__lineSpace(this: *mut Il2CppObject, lineSpace: f32)  {
    set_field_value(this, unsafe { LINESPACE_FIELD }, &lineSpace);
    _UpdateTextWrapper(this);
}

static mut FONTSIZE_FIELD: *mut FieldInfo = null_mut();
pub fn set__fontSize(this: *mut Il2CppObject, fontSize: i32) {
    set_field_value(this, unsafe { FONTSIZE_FIELD }, &fontSize);
    _UpdateTextWrapper(this);
}

static mut TEXT_OFFSET_FIELD: *mut FieldInfo = null_mut();
pub fn set__textOffset(this: *mut Il2CppObject, offset: Vector2_t) {
    set_field_value(this, unsafe { TEXT_OFFSET_FIELD }, &offset);
    _UpdatePosition(this);
}

static mut TEXT_FIELD: *mut FieldInfo = null_mut();

pub fn get__text(this: *mut Il2CppObject) -> *mut Il2CppString {
    let field = unsafe { TEXT_FIELD };
    if this.is_null() || field.is_null() { return null_mut(); }
    get_field_object_value(this, field)
}

fn set__text(this: *mut Il2CppObject, value: *mut Il2CppString) {
    set_field_object_value(this, unsafe { TEXT_FIELD }, value);
}

// pc/droid function signatures
#[cfg(target_os = "android")]
type _UpdatePositionFn = extern "C" fn(this: *mut Il2CppObject);
#[cfg(not(target_os = "android"))]
type _UpdatePositionFn = extern "C" fn(this: *mut Il2CppObject, method: usize);

#[cfg(target_os = "android")]
type _UpdateTextFn = extern "C" fn(this: *mut Il2CppObject);
#[cfg(not(target_os = "android"))]
type _UpdateTextFn = extern "C" fn(this: *mut Il2CppObject, method: usize);

// public wrapper functions
static mut _UPDATE_POSITION_ADDR: usize = 0;
pub fn _UpdatePosition(this: *mut Il2CppObject) {
    if this.is_null() || unsafe { _UPDATE_POSITION_ADDR } == 0 { return; }
    unsafe {
        #[cfg(target_os = "android")]
        {
            let orig_fn: _UpdatePositionFn = std::mem::transmute(_UPDATE_POSITION_ADDR);
            orig_fn(this);
        }
        #[cfg(not(target_os = "android"))]
        {
            let orig_fn: _UpdatePositionFn = std::mem::transmute(_UPDATE_POSITION_ADDR);
            orig_fn(this, _UPDATE_POSITION_ADDR);
        }
    }
}

static mut _UPDATE_TEXT_ADDR: usize = 0;
fn _UpdateTextWrapper(this: *mut Il2CppObject) {
    if this.is_null() || unsafe { _UPDATE_TEXT_ADDR } == 0 { return; }
    unsafe {
        #[cfg(target_os = "android")]
        {
            let orig_fn: _UpdateTextFn = std::mem::transmute(_UPDATE_TEXT_ADDR);
            orig_fn(this);
        }
        #[cfg(not(target_os = "android"))]
        {
            let orig_fn: _UpdateTextFn = std::mem::transmute(_UPDATE_TEXT_ADDR);
            orig_fn(this, _UPDATE_TEXT_ADDR);
        }
    }
}

// hooks

type SetTextFn = extern "C" fn(this: *mut Il2CppObject, text: *mut Il2CppString);
extern "C" fn SetText(this: *mut Il2CppObject, mut text: *mut Il2CppString) {
    let text_utf = unsafe { (*text).as_utf16str() };
    if !text_utf.as_slice().contains(&36) { // 36 = dollar sign ($)
        return get_orig_fn!(SetText, SetTextFn)(this, text);
    }

    // Rationale: AnText has fields and functions. The functions set the fields + update display.
    // Setting fields alone does not update current display, but does use them next time.
    // We store state, possibly modify current through templates, and restore state for next use.
    let line_space = get_lineSpace(this);
    let anchor = get_textAnchor(this);
    let font_size = get_fontSize(this);
    let fit = get_useFit(this);

    text = Hachimi::instance()
        .template_parser
        .eval_with_context(&text_utf.to_string(), &mut TemplateContext { component: this })
        .to_il2cpp_string();
    get_orig_fn!(SetText, SetTextFn)(this, text);

    set_lineSpace(this, line_space);
    set_textAnchor(this, anchor);
    set_fontSize(this, font_size);
    set_useFit(this, fit);
}

struct TemplateContext {
    component: *mut Il2CppObject,
}

impl template::Context for TemplateContext {
    fn on_filter_eval(&mut self, name: &str, args: &[template::Token]) -> Option<String> {
        match name {
            "anchor" => {
                let value = args.get(0)?;
                let template::Token::NumberLit(anchor_num) = *value else {
                    return None;
                };
                let anchor = match anchor_num as i32 - 1 {
                    0 => TextAnchor::UpperLeft,
                    1 => TextAnchor::UpperCenter,
                    2 => TextAnchor::UpperRight,
                    3 => TextAnchor::MiddleLeft,
                    4 => TextAnchor::MiddleCenter,
                    5 => TextAnchor::MiddleRight,
                    6 => TextAnchor::LowerLeft,
                    7 => TextAnchor::LowerCenter,
                    8 => TextAnchor::LowerRight,
                    _ => return Some(String::new()),
                };
                SetTextAnchor(self.component, anchor);
            }

            "scale" => {
                let value = args.get(0)?;
                let template::Token::NumberLit(percentage) = value else {
                    return None;
                };
                let cur_size = get_fontSize(self.component);
                let new_size = (cur_size as f64 * (percentage / 100.0)) as i32;
                SetTextFontSize(self.component, new_size);
                SetTextFit(self.component, false);
            }

            "ls" => {
                let value = args.get(0)?;
                let template::Token::NumberLit(ls) = *value else {
                    return None;
                };
                SetTextLinespace(self.component, ls as f32);
            }

            "bf" => {
                let value = args.get(0)?;
                let template::Token::NumberLit(state) = *value else {
                    return None;
                };
                SetTextFit(self.component, state != 0.0);
            }

            "bestfit" => {
                let value = args.get(0)?;
                let template::Token::NumberLit(state) = *value else {
                    return None;
                };
                SetTextFit(self.component, state != 0.0);
            }

            "wrap" => {
                let state = args.get(0)?;
                let template::Token::NumberLit(state) = *state else {
                    return None;
                };
                SetTextWrap(self.component, state != 0.0);
            }

            _ => return Some(String::new()),
        }

        Some(String::new())
    }
}

// Context that ignores AnText filters
pub struct IgnoreATFiltersContext();

impl template::Context for IgnoreATFiltersContext {
    fn on_filter_eval(&mut self, _name: &str, _args: &[template::Token]) -> Option<String> {
        match _name {
            "anchor" | "scale" | "ls" | "bf" | "bestfit" | "wrap" => Some(String::new()),
            _ => None
        }
    }
}

#[cfg(target_os = "android")]
extern "C" fn _UpdateText(this: *mut Il2CppObject) {
    let text_ptr = get__text(this);
    if text_ptr.is_null() {
        return get_orig_fn!(_UpdateText, _UpdateTextFn)(this);
    }

    let text = unsafe { (*text_ptr).as_utf16str() };
    let has_template = text.as_slice().contains(&36); // 36 = '$'
    let has_story_title = match crate::il2cpp::hook::umamusume::PartsSingleModeStoryEventTitle::LAST_STORY_EVENT_TITLE.read() {
        Ok(s) => !s.is_empty(),
        Err(_) => false,
    };

    if has_story_title || has_template {
        let text_str = text.to_string();

        if has_story_title {
            if let Ok(last_title) = crate::il2cpp::hook::umamusume::PartsSingleModeStoryEventTitle::LAST_STORY_EVENT_TITLE.read() {
                if *last_title == text_str && text_str.contains('\n') {
                    if let Some(config) = Hachimi::instance().localized_data.load().config.story_event_title.clone() {
                        if let Some(font_size) = config.font_size {
                            set_field_value(this, unsafe { FONTSIZE_FIELD }, &font_size);
                        }
                        if let Some(line_spacing) = config.line_spacing {
                            set_field_value(this, unsafe { LINESPACE_FIELD }, &line_spacing);
                        }
                        let offset = Vector2_t {
                            x: config.position_offset_x.unwrap_or(0.0),
                            y: config.position_offset_y.unwrap_or(0.0)
                        };
                        set_field_value(this, unsafe { TEXT_OFFSET_FIELD }, &offset);
                        _UpdatePosition(this);
                    }
                }
            }
        }

        if has_template {
            set__text(this, Hachimi::instance().template_parser
                .eval_with_context(&text_str, &mut IgnoreTGFiltersContext())
                .to_il2cpp_string());
        }
    }

    get_orig_fn!(_UpdateText, _UpdateTextFn)(this);
}

#[cfg(not(target_os = "android"))]
extern "C" fn _UpdateText(this: *mut Il2CppObject, method: usize) {
    let text_ptr = get__text(this);
    if text_ptr.is_null() {
        return get_orig_fn!(_UpdateText, _UpdateTextFn)(this, method);
    }

    let text = unsafe { (*text_ptr).as_utf16str() };
    let has_template = text.as_slice().contains(&36); // 36 = '$'
    let has_story_title = match crate::il2cpp::hook::umamusume::PartsSingleModeStoryEventTitle::LAST_STORY_EVENT_TITLE.read() {
        Ok(s) => !s.is_empty(),
        Err(_) => false,
    };

    if has_story_title || has_template {
        let text_str = text.to_string();

        if has_story_title {
            if let Ok(last_title) = crate::il2cpp::hook::umamusume::PartsSingleModeStoryEventTitle::LAST_STORY_EVENT_TITLE.read() {
                if *last_title == text_str && text_str.contains('\n') {
                    if let Some(config) = Hachimi::instance().localized_data.load().config.story_event_title.clone() {
                        if let Some(font_size) = config.font_size {
                            set_field_value(this, unsafe { FONTSIZE_FIELD }, &font_size);
                        }
                        if let Some(line_spacing) = config.line_spacing {
                            set_field_value(this, unsafe { LINESPACE_FIELD }, &line_spacing);
                        }
                        let offset = Vector2_t {
                            x: config.position_offset_x.unwrap_or(0.0),
                            y: config.position_offset_y.unwrap_or(0.0)
                        };
                        set_field_value(this, unsafe { TEXT_OFFSET_FIELD }, &offset);
                        _UpdatePosition(this);
                    }
                }
            }
        }

        if has_template {
            set__text(this, Hachimi::instance().template_parser
                .eval_with_context(&text_str, &mut IgnoreTGFiltersContext())
                .to_il2cpp_string());
        }
    }

    get_orig_fn!(_UpdateText, _UpdateTextFn)(this, method);
}

pub fn init(image: *const Il2CppImage) {
    get_class_or_return!(image, AnimateToUnity, AnText);

    let _UpdateText_addr = get_method_addr(AnText, c"_UpdateText", 0);
    new_hook!(_UpdateText_addr, _UpdateText);

    let SetText_addr = get_method_addr(AnText, c"SetText", 1);
    new_hook!(SetText_addr, SetText);

    unsafe {
        TEXT_OFFSET_FIELD = get_field_from_name(AnText, c"_textOffset");
        LINESPACE_FIELD = get_field_from_name(AnText, c"_lineSpace");
        FONTSIZE_FIELD = get_field_from_name(AnText, c"_fontSize");
        TEXT_FIELD = get_field_from_name(AnText, c"_text");
        TEXTANCHOR_FIELD = get_field_from_name(AnText, c"_textAnchor");
        USEFIT_FIELD = get_field_from_name(AnText, c"_useFit");
        USEWRAP_FIELD = get_field_from_name(AnText, c"_useWrap");
        _UPDATE_TEXT_ADDR = _UpdateText_addr;
        _UPDATE_POSITION_ADDR = get_method_addr(AnText, c"_UpdatePosition", 0);
        SET_TEXT_FIT_ADDR = get_method_addr(AnText, c"SetTextFit", 1);
        SET_TEXT_WRAP_ADDR = get_method_addr(AnText, c"SetTextWrap", 1);
        SET_TEXT_ANCHOR_ADDR = get_method_addr(AnText, c"SetTextAnchor", 1);
        SET_TEXT_LINESPACE_ADDR = get_method_addr(AnText, c"SetTextLinespace", 1);
        SET_TEXT_FONTSIZE_ADDR = get_method_addr(AnText, c"SetTextFontSize", 1);
    }
}