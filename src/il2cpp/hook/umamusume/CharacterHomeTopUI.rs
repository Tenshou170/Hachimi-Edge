use crate::il2cpp::{symbols::{get_field_value, get_field_from_name, get_field_from_name_opt}, types::*};

use std::ptr::null_mut;

static mut CARD_ROOT_BUTTON_FIELD: *mut FieldInfo = null_mut();
static mut SUPPORT_CARD_ROOT_BUTTON_FIELD: *mut FieldInfo = null_mut();
static mut TRAINED_CHARA_ROOT_BUTTON_FIELD: *mut FieldInfo = null_mut();
static mut CHARACTER_CARD_CATALOG_BUTTON_FIELD: *mut FieldInfo = null_mut();
static mut CARD_LV_UP_BUTTON_FIELD: *mut FieldInfo = null_mut();
static mut HINT_LV_UP_BUTTON_FIELD: *mut FieldInfo = null_mut();
static mut CARD_LIMIT_BREAK_BUTTON_FIELD: *mut FieldInfo = null_mut();
static mut PIECE_EXCHANGE_BUTTON_FIELD: *mut FieldInfo = null_mut();
static mut SUPPORT_EDIT_BUTTON_FIELD: *mut FieldInfo = null_mut();
static mut SUPPORT_SELL_BUTTON_FIELD: *mut FieldInfo = null_mut();
static mut SUPPORT_LIST_BUTTON_FIELD: *mut FieldInfo = null_mut();
static mut TRAINED_LIST_BUTTON_FIELD: *mut FieldInfo = null_mut();
static mut NEW_TEAM_EDIT_BUTTON_FIELD: *mut FieldInfo = null_mut();
static mut SUPPORT_LV_UP_BUTTON_FIELD: *mut FieldInfo = null_mut();
static mut SUPPORT_LIMIT_BREAK_BUTTON_FIELD: *mut FieldInfo = null_mut();
static mut TEAM_EDIT_BUTTON_FIELD: *mut FieldInfo = null_mut();
static mut TRANSFER_BUTTON_FIELD: *mut FieldInfo = null_mut();
static mut TRAINED_CHARA_ROOT_SHORT_BUTTON_FIELD: *mut FieldInfo = null_mut();
static mut SUCCESSION_ONLY_CHARA_ROOT_BUTTON_FIELD: *mut FieldInfo = null_mut();
static mut SUCCESSION_ONLY_START_BUTTON_FIELD: *mut FieldInfo = null_mut();
static mut SUCCESSION_ONLY_LIST_BUTTON_FIELD: *mut FieldInfo = null_mut();

// public getter functions
pub fn get_cardRootButton(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_value(this, unsafe { CARD_ROOT_BUTTON_FIELD })
}

pub fn get_supportCardRootButton(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_value(this, unsafe { SUPPORT_CARD_ROOT_BUTTON_FIELD })
}

pub fn get_trainedCharaRootButton(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_value(this, unsafe { TRAINED_CHARA_ROOT_BUTTON_FIELD })
}

pub fn get_characterCardCatalogButton(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_value(this, unsafe { CHARACTER_CARD_CATALOG_BUTTON_FIELD })
}

pub fn get_cardLvUpButton(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_value(this, unsafe { CARD_LV_UP_BUTTON_FIELD })
}

pub fn get_hintLvUpButton(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_value(this, unsafe { HINT_LV_UP_BUTTON_FIELD })
}

pub fn get_cardLimitBreakButton(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_value(this, unsafe { CARD_LIMIT_BREAK_BUTTON_FIELD })
}

pub fn get_pieceExchangeButton(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_value(this, unsafe { PIECE_EXCHANGE_BUTTON_FIELD })
}

pub fn get_supportEditButton(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_value(this, unsafe { SUPPORT_EDIT_BUTTON_FIELD })
}

pub fn get_supportSellButton(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_value(this, unsafe { SUPPORT_SELL_BUTTON_FIELD })
}

pub fn get_supportListButton(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_value(this, unsafe { SUPPORT_LIST_BUTTON_FIELD })
}

pub fn get_trainedListButton(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_value(this, unsafe { TRAINED_LIST_BUTTON_FIELD })
}

pub fn get_supportLvUpButton(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_value(this, unsafe { SUPPORT_LV_UP_BUTTON_FIELD })
}

pub fn get_supportLimitBreakButton(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_value(this, unsafe { SUPPORT_LIMIT_BREAK_BUTTON_FIELD })
}

pub fn get_newTeamEditButton(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_value(this, unsafe { NEW_TEAM_EDIT_BUTTON_FIELD }) 
}

pub fn get_teamEditButton(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_value(this, unsafe { TEAM_EDIT_BUTTON_FIELD })
}

pub fn get_transferButton(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_value(this, unsafe { TRANSFER_BUTTON_FIELD }) 
}

pub fn get_trainedCharaRootShortButton(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_value(this, unsafe { TRAINED_CHARA_ROOT_SHORT_BUTTON_FIELD }) 
}

pub fn get_successionOnlyCharaRootButton(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_value(this, unsafe { SUCCESSION_ONLY_CHARA_ROOT_BUTTON_FIELD }) 
}

pub fn get_successionOnlyStartButton(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_value(this, unsafe { SUCCESSION_ONLY_START_BUTTON_FIELD }) 
}

pub fn get_successionOnlyListButton(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_value(this, unsafe { SUCCESSION_ONLY_LIST_BUTTON_FIELD }) 
}

pub fn init(Plugins: *const Il2CppImage) {
    get_class_or_return!(Plugins, Gallop, CharacterHomeTopUI);

    unsafe {
        CARD_ROOT_BUTTON_FIELD = get_field_from_name(CharacterHomeTopUI, c"_cardRootButton");
        SUPPORT_CARD_ROOT_BUTTON_FIELD = get_field_from_name(CharacterHomeTopUI, c"_supportCardRootButton");
        TRAINED_CHARA_ROOT_BUTTON_FIELD = get_field_from_name(CharacterHomeTopUI, c"_trainedCharaRootButton");
        CHARACTER_CARD_CATALOG_BUTTON_FIELD = get_field_from_name(CharacterHomeTopUI, c"_characterCardCatalogButton");
        CARD_LV_UP_BUTTON_FIELD = get_field_from_name(CharacterHomeTopUI, c"_cardLvUpButton");
        HINT_LV_UP_BUTTON_FIELD = get_field_from_name(CharacterHomeTopUI, c"_hintLvUpButton");
        CARD_LIMIT_BREAK_BUTTON_FIELD = get_field_from_name(CharacterHomeTopUI, c"_cardLimitBreakButton");
        PIECE_EXCHANGE_BUTTON_FIELD = get_field_from_name(CharacterHomeTopUI, c"_pieceExchangeButton");
        SUPPORT_EDIT_BUTTON_FIELD = get_field_from_name(CharacterHomeTopUI, c"_supportEditButton");
        SUPPORT_SELL_BUTTON_FIELD = get_field_from_name(CharacterHomeTopUI, c"_supportSellButton");
        SUPPORT_LIST_BUTTON_FIELD = get_field_from_name(CharacterHomeTopUI, c"_supportListButton");
        TRAINED_LIST_BUTTON_FIELD = get_field_from_name(CharacterHomeTopUI, c"_trainedListButton");

        NEW_TEAM_EDIT_BUTTON_FIELD = get_field_from_name_opt(CharacterHomeTopUI, c"_newTeamEditButton").unwrap_or(null_mut());
        TRANSFER_BUTTON_FIELD = get_field_from_name_opt(CharacterHomeTopUI, c"_transferButton").unwrap_or(null_mut());
        TRAINED_CHARA_ROOT_SHORT_BUTTON_FIELD = get_field_from_name_opt(CharacterHomeTopUI, c"_trainedCharaRootShortButton").unwrap_or(null_mut());
        SUCCESSION_ONLY_CHARA_ROOT_BUTTON_FIELD = get_field_from_name_opt(CharacterHomeTopUI, c"_successionOnlyCharaRootButton").unwrap_or(null_mut());
        SUCCESSION_ONLY_START_BUTTON_FIELD = get_field_from_name_opt(CharacterHomeTopUI, c"_successionOnlyStartButton").unwrap_or(null_mut());
        SUCCESSION_ONLY_LIST_BUTTON_FIELD = get_field_from_name_opt(CharacterHomeTopUI, c"_successionOnlyListButton").unwrap_or(null_mut());
        TEAM_EDIT_BUTTON_FIELD = get_field_from_name_opt(CharacterHomeTopUI, c"_teamEditButton").unwrap_or(null_mut());
        SUPPORT_LV_UP_BUTTON_FIELD = get_field_from_name_opt(CharacterHomeTopUI, c"_supportLvUpButton").unwrap_or(null_mut());
        SUPPORT_LIMIT_BREAK_BUTTON_FIELD = get_field_from_name_opt(CharacterHomeTopUI, c"_supportLimitBreakButton").unwrap_or(null_mut());
    }
}