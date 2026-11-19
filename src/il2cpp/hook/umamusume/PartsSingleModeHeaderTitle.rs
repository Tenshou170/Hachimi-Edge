use crate::il2cpp::{symbols::get_method_addr, types::*};

use super::PartsCommonHeaderTitle::get_label;

extern "C" fn GetInLabel(_this: *mut Il2CppObject, text: *mut Il2CppString) -> *mut Il2CppString {
    get_label(text)
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, PartsSingleModeHeaderTitle);
    find_nested_class_or_return!(PartsSingleModeHeaderTitle, SingleModeTitlePlayer);

    let GetInLabel_addr = get_method_addr(SingleModeTitlePlayer, c"GetInLabel", 1);
    if GetInLabel_addr != 0 {
        new_hook!(GetInLabel_addr, GetInLabel);
    }
}
