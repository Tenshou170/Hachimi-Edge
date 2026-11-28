use crate::il2cpp::{
    hook::{
        UnityEngine_CoreModule::{Behaviour, Component, Object, RectTransform},
        UnityEngine_UI::Text,
    },
    symbols::{get_field_from_name, get_field_object_value, get_method_addr},
    types::*,
};

static mut NAME_CONTENTS_SIZE_FITTER_FIELD: *mut FieldInfo = 0 as _;
pub fn get_nameContentsSizeFitter(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_object_value(this, unsafe { NAME_CONTENTS_SIZE_FITTER_FIELD })
}

static mut NAME_TEXT_FIELD: *mut FieldInfo = 0 as _;
pub fn get_nameText(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_object_value(this, unsafe { NAME_TEXT_FIELD })
}

type SetUpCharacterLimitBreakSkillFn =
    extern "C" fn(this: *mut Il2CppObject, cardRairtyData: *mut Il2CppObject, nextCardRairtyData: *mut Il2CppObject, atlas: *mut Il2CppObject);
extern "C" fn SetUpCharacterLimitBreakSkill(
    this: *mut Il2CppObject,
    cardRairtyData: *mut Il2CppObject,
    nextCardRairtyData: *mut Il2CppObject,
    atlas: *mut Il2CppObject,
) {
    get_orig_fn!(SetUpCharacterLimitBreakSkill, SetUpCharacterLimitBreakSkillFn)(this, cardRairtyData, nextCardRairtyData, atlas);

    if this.is_null() || !Object::op_Implicit(this) {
        return;
    }
    let text = get_nameText(this);
    if !text.is_null() && Object::op_Implicit(text) {
        let fitter = get_nameContentsSizeFitter(this);
        if !fitter.is_null() && Object::op_Implicit(fitter) {
            Behaviour::set_enabled(fitter, false);
        }
        Text::set_resizeTextForBestFit(text, true);
        Text::set_resizeTextMaxSize(text, 26);

        let text_tr = Component::get_transform(text);
        if !text_tr.is_null() {
            RectTransform::set_sizeDelta(text_tr, Vector2_t { x: 320.0, y: 68.0 });

            let mut anchored_pos = RectTransform::get_anchoredPosition(text_tr);
            anchored_pos.y = 3.29993;
            RectTransform::set_anchoredPosition(text_tr, anchored_pos);
        }
    }
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, PartsGetSkillPlate);
    get_class_or_return!(umamusume, Gallop, PartsGetStatusPlate); // Base class

    let SetUpCharacterLimitBreakSkill_addr = get_method_addr(PartsGetSkillPlate, c"SetUpCharacterLimitBreakSkill", 3);
    new_hook!(SetUpCharacterLimitBreakSkill_addr, SetUpCharacterLimitBreakSkill);

    unsafe {
        NAME_CONTENTS_SIZE_FITTER_FIELD = get_field_from_name(PartsGetSkillPlate, c"_nameContentsSizeFitter");
        NAME_TEXT_FIELD = get_field_from_name(PartsGetStatusPlate, c"_nameText");
    }
}
