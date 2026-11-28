use crate::{
    core::{game::Region, Hachimi},
    il2cpp::{
        ext::Il2CppObjectExt,
        hook::{
            UnityEngine_CoreModule::{Component, Object, RectTransform},
            UnityEngine_UI::Text,
        },
        sql::TextDataQuery,
        symbols::{get_field_from_name, get_field_object_value, get_method_addr},
        types::*,
    },
};

// SkillListItem
static mut NAMETEXT_FIELD: *mut FieldInfo = 0 as _;
pub fn get__nameText(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_object_value(this, unsafe { NAMETEXT_FIELD })
}
static mut DESCTEXT_FIELD: *mut FieldInfo = 0 as _;
pub fn get__descText(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_object_value(this, unsafe { DESCTEXT_FIELD })
}

static mut INFO_FIELD: *mut FieldInfo = 0 as _;
pub fn get_info(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_object_value(this, unsafe { INFO_FIELD })
}

static mut set_skill_name_text_addr: usize = 0;
impl_addr_wrapper_fn!(set_skill_name_text, set_skill_name_text_addr, (), this: *mut Il2CppObject);

// PartsSingleModeSkillListItem.Info
static mut get_IsDrawDesc_addr: usize = 0;
impl_addr_wrapper_fn!(get_IsDrawDesc, get_IsDrawDesc_addr, bool, this: *mut Il2CppObject);
static mut get_IsDrawNeedSkillPoint_addr: usize = 0;
impl_addr_wrapper_fn!(get_IsDrawNeedSkillPoint, get_IsDrawNeedSkillPoint_addr, bool, this: *mut Il2CppObject);

fn UpdateItemCommon(this: *mut Il2CppObject, skill_info: *mut Il2CppObject, orig_fn_cb: impl FnOnce()) {
    TextDataQuery::with_skill_learning_query(|| {
        orig_fn_cb();
    });

    let name = get__nameText(this);
    let desc = get__descText(this);
    let is_small_jp = Hachimi::instance().game.region == Region::Japan
        && !this.is_null()
        && unsafe { &*this }.name() == "PartsSingleModeSkillListItem_S";

    // Name should always exist, but let's be sure.
    if !name.is_null() && Object::op_Implicit(name) {
        Text::set_horizontalOverflow(name, 0);
        Text::set_resizeTextForBestFit(name, true);
        if is_small_jp {
            Text::set_resizeTextMaxSize(name, 26);
        }
    }

    if !skill_info.is_null() && get_IsDrawDesc(skill_info) && !desc.is_null() && Object::op_Implicit(desc) {
        Text::set_horizontalOverflow(desc, 0);
        Text::set_resizeTextForBestFit(desc, true);
        Text::set_resizeTextMinSize(desc, 14);
        Text::set_resizeTextMaxSize(desc, 30);
    }

    if is_small_jp && !name.is_null() {
        let name_tr = Component::get_transform(name);
        if !name_tr.is_null() {
            RectTransform::set_sizeDelta(name_tr, Vector2_t { x: -172.0, y: 68.0 });

            let mut anchored_pos = RectTransform::get_anchoredPosition(name_tr);
            anchored_pos.y = 3.29993;
            RectTransform::set_anchoredPosition(name_tr, anchored_pos);
        }
    }

    if let Some(mult) = Hachimi::instance().localized_data.load().config.skill_list_item_desc_font_size_multiplier {
        let desc_text = get__descText(this);
        if !desc_text.is_null() && Object::op_Implicit(desc_text) {
            let font_size = Text::get_fontSize(desc_text);
            Text::set_fontSize(desc_text, (font_size as f32 * mult).round() as i32);
        }
    }
}

type UpdateItemJpFn = extern "C" fn(this: *mut Il2CppObject, skill_info: *mut Il2CppObject, is_plate_effect_enable: bool, adjuster_data: *mut Il2CppObject, resource_hash: i32, on_click_button: *mut Il2CppObject);
extern "C" fn UpdateItemJp(this: *mut Il2CppObject, skill_info: *mut Il2CppObject, is_plate_effect_enable: bool, adjuster_data: *mut Il2CppObject, resource_hash: i32, on_click_button: *mut Il2CppObject) {
    UpdateItemCommon(this, skill_info, || {
        get_orig_fn!(UpdateItemJp, UpdateItemJpFn)(this, skill_info, is_plate_effect_enable, adjuster_data, resource_hash, on_click_button);
    });
}

type UpdateItemGlobalTwFn = extern "C" fn(this: *mut Il2CppObject, skill_info: *mut Il2CppObject, is_plate_effect_enable: bool, resource_hash: i32);
extern "C" fn UpdateItemGlobalTw(this: *mut Il2CppObject, skill_info: *mut Il2CppObject, is_plate_effect_enable: bool, resource_hash: i32) {
    UpdateItemCommon(this, skill_info, || {
        get_orig_fn!(UpdateItemGlobalTw, UpdateItemGlobalTwFn)(this, skill_info, is_plate_effect_enable, resource_hash);
    });
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, PartsSingleModeSkillListItem);
    find_nested_class_or_return!(PartsSingleModeSkillListItem, Info);

    let UpdateItem5_addr = get_method_addr(PartsSingleModeSkillListItem, c"UpdateItem", 5);
    if UpdateItem5_addr != 0 {
        new_hook!(UpdateItem5_addr, UpdateItemJp);
    } else {
        let UpdateItem3_addr = get_method_addr(PartsSingleModeSkillListItem, c"UpdateItem", 3);
        if UpdateItem3_addr != 0 {
            new_hook!(UpdateItem3_addr, UpdateItemGlobalTw);
        }
    }

    unsafe {
        // PartsSingleModeSkillListItem
        NAMETEXT_FIELD = get_field_from_name(PartsSingleModeSkillListItem, c"_nameText");
        DESCTEXT_FIELD = get_field_from_name(PartsSingleModeSkillListItem, c"_descText");
        INFO_FIELD = get_field_from_name(PartsSingleModeSkillListItem, c"_info");
        set_skill_name_text_addr = get_method_addr(PartsSingleModeSkillListItem, c"SetSkillNameText", 0);

        // PartsSingleModeSkillListItem.Info
        get_IsDrawDesc_addr = get_method_addr(Info, c"get_IsDrawDesc", 0);
        get_IsDrawNeedSkillPoint_addr = get_method_addr(Info, c"get_IsDrawNeedSkillPoint", 0);
    }
}