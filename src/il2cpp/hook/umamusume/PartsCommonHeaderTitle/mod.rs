mod TitlePlayer;

use crate::{
    core::Hachimi,
    il2cpp::{
        ext::{Il2CppStringExt, StringExt},
        symbols::get_method_addr,
        types::*,
    },
};

fn clean_template(text: &str) -> String {
    if text.contains('$') {
        Hachimi::instance().template_parser.remove_filters(text)
    } else {
        text.to_string()
    }
}

pub fn get_label(text: *mut Il2CppString) -> *mut Il2CppString {
    if text.is_null() { return "in_min".to_il2cpp_string(); }

    let raw = unsafe { (*text).as_utf16str().to_string() };
    let text = clean_template(&raw);

    let mut len = 0.0f32;
    for ch in text.chars() {
        len += if ch.is_ascii() || (ch as u32) < 0x2E80 { 0.55 } else { 1.0 };
    }
    let len = len.ceil() as i32;

    let label = if len <= 8 { "in_min" } else if len <= 13 { "in" } else { "in_max" };

    label.to_il2cpp_string()
}

type SetNextText4Fn = extern "C" fn(this: *mut Il2CppObject, text: *mut Il2CppString, guide_id: i32, on_open_guide: *mut Il2CppObject, on_destroy: *mut Il2CppObject);
extern "C" fn SetNextText4(this: *mut Il2CppObject, text_: *mut Il2CppString, guide_id: i32, on_open_guide: *mut Il2CppObject, on_destroy: *mut Il2CppObject) {
    let clean_text = if !text_.is_null() {
        let s = unsafe { (*text_).as_utf16str().to_string() };
        clean_template(&s).to_il2cpp_string()
    } else {
        text_
    };

    get_orig_fn!(SetNextText4, SetNextText4Fn)(this, clean_text, guide_id, on_open_guide, on_destroy);
}

type SetNextText2Fn = extern "C" fn(this: *mut Il2CppObject, text: *mut Il2CppString, guide_id: i32);
extern "C" fn SetNextText2(this: *mut Il2CppObject, text_: *mut Il2CppString, guide_id: i32) {
    let clean_text = if !text_.is_null() {
        let s = unsafe { (*text_).as_utf16str().to_string() };
        clean_template(&s).to_il2cpp_string()
    } else {
        text_
    };

    get_orig_fn!(SetNextText2, SetNextText2Fn)(this, clean_text, guide_id);
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, PartsCommonHeaderTitle);

    let SetNextText4_addr = get_method_addr(PartsCommonHeaderTitle, c"SetNextText", 4);
    if SetNextText4_addr != 0 {
        new_hook!(SetNextText4_addr, SetNextText4);
    } else {
        let SetNextText2_addr = get_method_addr(PartsCommonHeaderTitle, c"SetNextText", 2);
        if SetNextText2_addr != 0 {
            new_hook!(SetNextText2_addr, SetNextText2);
        }
    }

    TitlePlayer::init(PartsCommonHeaderTitle);
}
