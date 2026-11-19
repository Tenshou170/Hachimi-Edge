use crate::{
    il2cpp::{
        ext::{Il2CppStringExt, StringExt},
        symbols::get_method_addr,
        types::*,
    },
};

use super::{clean_template, get_label};

pub fn init(PartsCommonHeaderTitle: *mut Il2CppClass) {
    find_nested_class_or_return!(PartsCommonHeaderTitle, TitlePlayer);

    let GetInMotionName_addr = get_method_addr(TitlePlayer, c"GetInMotionName", 1);
    if GetInMotionName_addr != 0 {
        new_hook!(GetInMotionName_addr, GetInMotionName);
    }

    let Play_addr = get_method_addr(TitlePlayer, c"Play", 2);
    if Play_addr != 0 {
        new_hook!(Play_addr, Play);
    }
}

extern "C" fn GetInMotionName(_this: *mut Il2CppObject, text: *mut Il2CppString) -> *mut Il2CppString {
    get_label(text)
}

type PlayFn = extern "C" fn(this: *mut Il2CppObject, text: *mut Il2CppString, callback: *mut Il2CppObject);
extern "C" fn Play(this: *mut Il2CppObject, text_: *mut Il2CppString, callback: *mut Il2CppObject) {
    let clean_text = if !text_.is_null() {
        let s = unsafe { (*text_).as_utf16str().to_string() };
        clean_template(&s).to_il2cpp_string()
    } else {
        text_
    };
    get_orig_fn!(Play, PlayFn)(this, clean_text, callback);
}
