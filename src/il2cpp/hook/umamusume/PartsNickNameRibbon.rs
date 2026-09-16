use crate::{
    core::{game::Region, Hachimi},
    il2cpp::{
        hook::{
            UnityEngine_CoreModule::{Behaviour, Component, GameObject},
            UnityEngine_TextRenderingModule::TextAnchor,
            UnityEngine_UI::{ContentSizeFitter, Text},
        },
        symbols::get_method_addr,
        types::*,
        utils,
    },
};

const WIDTH: f32 = 320.0;
const HEIGHT: f32 = 50.0;

static mut GET_LABEL_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_Label, GET_LABEL_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

use crate::il2cpp::hook::UnityEngine_CoreModule::Object;

type InitFn = extern "C" fn(this: *mut Il2CppObject, nickname: *mut Il2CppObject, isLandscape: bool);
extern "C" fn Initialize(this: *mut Il2CppObject, nickname: *mut Il2CppObject, isLandscape: bool) {
    get_orig_fn!(Initialize, InitFn)(this, nickname, isLandscape);
    if !this.is_null() && Object::op_Implicit(this) {
        fit_text(this);
    }
}

pub fn fit_text(ribbon: *mut Il2CppObject) {
    if ribbon.is_null() || !Object::op_Implicit(ribbon) {
        return;
    }
    let label = get_Label(ribbon);
    if label.is_null() || !Object::op_Implicit(label) {
        return;
    }
    let label_obj = Component::get_gameObject(label);
    if label_obj.is_null() || !Object::op_Implicit(label_obj) {
        return;
    }
    let fitter = GameObject::GetComponent(label_obj, ContentSizeFitter::type_object());
    if !fitter.is_null() && Object::op_Implicit(fitter) {
        // Disable ContentSizeFitter to stop auto-sizing to text needs.
        Behaviour::set_enabled(fitter, false);
    }

    // Set the text bounds to something sane.
    utils::adjust_transform_size(label, WIDTH, HEIGHT);

    Text::set_best_fit_downscale(label);
    Text::set_alignment(label, TextAnchor::MiddleCenter);
}

pub fn init(umamusume: *const Il2CppImage) {
    if Hachimi::instance().game.region != Region::Japan {
        return;
    }

    get_class_or_return!(umamusume, Gallop, PartsNickNameRibbon);

    let initialize_addr = get_method_addr(PartsNickNameRibbon, c"Initialize", 2);
    new_hook!(initialize_addr, Initialize);

    unsafe {
        GET_LABEL_ADDR = get_method_addr(PartsNickNameRibbon, c"get_Label", 0);
    }
}
