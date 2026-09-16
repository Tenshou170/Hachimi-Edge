use crate::{
    core::{game::Region, Hachimi},
    il2cpp::{
        hook::umamusume::PartsNickNameRibbon,
        symbols::{get_field_from_name, get_field_object_value, get_method_addr},
        types::*,
    },
};

static mut RIBBON_FIELD: *mut FieldInfo = 0 as _;
pub fn get_ribbon(this: *mut Il2CppObject) -> *mut Il2CppObject {
    get_field_object_value(this, unsafe { RIBBON_FIELD })
}

use crate::il2cpp::hook::UnityEngine_CoreModule::Object;

type SetupFn = extern "C" fn(this: *mut Il2CppObject, nickNameId: i32, onSelect: *mut Il2CppDelegate);
extern "C" fn Setup(this: *mut Il2CppObject, nickNameId: i32, onSelect: *mut Il2CppDelegate) {
    get_orig_fn!(Setup, SetupFn)(this, nickNameId, onSelect);
    if !this.is_null() && Object::op_Implicit(this) {
        let ribbon = get_ribbon(this);
        if !ribbon.is_null() && Object::op_Implicit(ribbon) {
            PartsNickNameRibbon::fit_text(ribbon);
        }
    }
}

pub fn init(umamusume: *const Il2CppImage) {
    if Hachimi::instance().game.region != Region::Japan {
        return;
    }

    get_class_or_return!(umamusume, Gallop, PartsNickNameListItem);

    let initialize_addr = get_method_addr(PartsNickNameListItem, c"Setup", 2);
    new_hook!(initialize_addr, Setup);

    unsafe {
        RIBBON_FIELD = get_field_from_name(PartsNickNameListItem, c"_ribbon");
    }
}
