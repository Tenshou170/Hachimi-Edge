use serde::{Deserialize, Serialize};

use crate::{
    core::capabilities::{self, Capability},
    il2cpp::symbols::get_field_from_name_opt,
    il2cpp::types::*,
};

#[derive(Default, Copy, Clone, Serialize, Deserialize, Eq, PartialEq, Debug)]
#[repr(i32)]
pub enum ShadowType3d {
    #[default]
    Default = -1,
    None = 0,
    CircleShadow = 1,
    HardShadow = 2,
    SoftShadow = 3
}

impl ShadowType3d {
    pub fn is_enabled(&self) -> bool {
        *self != ShadowType3d::Default
    }
}

static mut CLASS: *mut Il2CppClass = 0 as _;
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

def_field_value_accessors!(set set_ShadowType, SHADOWTYPE_FIELD, ShadowType3d);

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, StoryTimelineBg3DClipData);

    unsafe {
        CLASS = StoryTimelineBg3DClipData;
        if let Some(field) = get_field_from_name_opt(StoryTimelineBg3DClipData, c"ShadowType") {
            SHADOWTYPE_FIELD = field;
            capabilities::set(Capability::StoryShadowType);
        }
    }
}
