use crate::il2cpp::{
    hook::UnityEngine_UI::Text,
    sql::{self, TextDataQuery},
    symbols::{get_field_from_name, get_field_object_value, get_method_addr},
    types::*,
};

static mut NAMETEXT_FIELD: *mut FieldInfo = 0 as _;
fn get__nameText(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_object_value(this, unsafe { NAMETEXT_FIELD })
}
static mut DESCTEXT_FIELD: *mut FieldInfo = 0 as _;
fn get__descText(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_object_value(this, unsafe { DESCTEXT_FIELD })
}

type UpdateCurrentFn = extern "C" fn(this: *mut Il2CppObject);
extern "C" fn UpdateCurrent(this: *mut Il2CppObject) {
    let name = get__nameText(this);
    let desc = get__descText(this);

    // Apply best-fit settings unconditionally before the original call so the
    // text components are ready regardless of whether a translation is found.
    // Matches Reko's approach: let Unity handle layout, GallopUtil is suppressed
    // via TDQ_IS_SKILL_LEARNING_QUERY so it doesn't double-wrap.
    if !name.is_null() {
        Text::set_horizontalOverflow(name, 0);
        Text::set_resizeTextForBestFit(name, true);
    }
    if !desc.is_null() {
        Text::set_horizontalOverflow(desc, 0);
        Text::set_resizeTextForBestFit(desc, true);
        Text::set_resizeTextMinSize(desc, 14);
        Text::set_resizeTextMaxSize(desc, 30);
    }

    let mut skill_cfg = sql::SkillTextFormatting::default();
    if !name.is_null() {
        skill_cfg.name = Some(sql::TextFormatting {
            line_len: 13,
            line_count: 1,
            font_size: Text::get_fontSize(name),
        });
    }
    if !desc.is_null() {
        skill_cfg.desc = Some(sql::TextFormatting {
            line_len: 18,
            line_count: 4,
            font_size: Text::get_fontSize(desc),
        });
    }

    TextDataQuery::with_skill_query(&skill_cfg, || {
        TextDataQuery::with_skill_learning_query(|| {
            get_orig_fn!(UpdateCurrent, UpdateCurrentFn)(this);
        });
    });

    // If a translation was found, allow overflow so the fitted text isn't clipped.
    if skill_cfg.is_localized {
        if !name.is_null() {
            Text::set_horizontalOverflow(name, 1);
        }
        if !desc.is_null() {
            Text::set_horizontalOverflow(desc, 1);
        }
    }
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, PartsSingleModeSkillLearningListItem);

    let UpdateCurrent_addr =
        get_method_addr(PartsSingleModeSkillLearningListItem, c"UpdateCurrent", 0);

    new_hook!(UpdateCurrent_addr, UpdateCurrent);

    unsafe {
        NAMETEXT_FIELD =
            get_field_from_name(PartsSingleModeSkillLearningListItem, c"_nameText");
        DESCTEXT_FIELD =
            get_field_from_name(PartsSingleModeSkillLearningListItem, c"_descriptionText");
    }
}
