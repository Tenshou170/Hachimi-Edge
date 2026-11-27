use std::ptr::null_mut;

use crate::{
    core::{game::Region, Hachimi},
    il2cpp::{
        api::il2cpp_class_is_assignable_from,
        ext::Il2CppObjectExt,
        hook::{
            UnityEngine_CoreModule::{Component, RectTransform},
            UnityEngine_UI::Text,
        },
        symbols::get_method_addr,
        types::*,
        utils::get_text_component,
    },
};

static mut CLASS: *mut Il2CppClass = null_mut();
fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

struct TextLayout {
    path: &'static str,
    size_delta_x: f32,
    anchored_pos_x: Option<f32>,
    anchored_pos_y: f32,
    max_size: i32,
}

const TEXT_LAYOUTS: [TextLayout; 13] = [
    TextLayout { path: "馬場適性", size_delta_x: 170.0, anchored_pos_x: None, anchored_pos_y: 46.53, max_size: 28 },
    TextLayout { path: "距離適性", size_delta_x: 170.0, anchored_pos_x: None, anchored_pos_y: 2.66, max_size: 28 },
    TextLayout { path: "走法適性", size_delta_x: 170.0, anchored_pos_x: None, anchored_pos_y: -39.25, max_size: 28 },
    TextLayout { path: "芝", size_delta_x: 138.0, anchored_pos_x: Some(-222.0), anchored_pos_y: 44.0, max_size: 34 },
    TextLayout { path: "ダート", size_delta_x: 158.0, anchored_pos_x: Some(-38.0), anchored_pos_y: 44.0, max_size: 34 },
    TextLayout { path: "短距離", size_delta_x: 138.0, anchored_pos_x: Some(-222.0), anchored_pos_y: 3.0, max_size: 34 },
    TextLayout { path: "マイル", size_delta_x: 158.0, anchored_pos_x: Some(-38.0), anchored_pos_y: 3.0, max_size: 34 },
    TextLayout { path: "中距離", size_delta_x: 158.0, anchored_pos_x: Some(160.0), anchored_pos_y: 3.0, max_size: 34 },
    TextLayout { path: "長距離", size_delta_x: 158.0, anchored_pos_x: Some(355.0), anchored_pos_y: 3.0, max_size: 34 },
    TextLayout { path: "逃げ", size_delta_x: 138.0, anchored_pos_x: Some(-222.0), anchored_pos_y: -42.0, max_size: 34 },
    TextLayout { path: "先行", size_delta_x: 158.0, anchored_pos_x: Some(-38.0), anchored_pos_y: -42.0, max_size: 34 },
    TextLayout { path: "差し", size_delta_x: 158.0, anchored_pos_x: Some(160.0), anchored_pos_y: -42.0, max_size: 34 },
    TextLayout { path: "追込", size_delta_x: 158.0, anchored_pos_x: Some(355.0), anchored_pos_y: -42.0, max_size: 34 },
];

fn on_setup(this: *mut Il2CppObject) {
    if this.is_null() {
        return;
    }

    let tr = Component::get_transform(this);
    if tr.is_null() {
        return;
    }

    for layout in &TEXT_LAYOUTS {
        let text = get_text_component(tr, layout.path);
        if text.is_null() {
            warn!("[PartsSingleModeStartProperFrame] Text not found: {}", layout.path);
            continue;
        }
        Text::set_resizeTextForBestFit(text, true);
        Text::set_resizeTextMaxSize(text, layout.max_size);

        let text_tr = Component::get_transform(text);
        if !text_tr.is_null() {
            let mut size_delta = RectTransform::get_sizeDelta(text_tr);
            size_delta.x = layout.size_delta_x;
            RectTransform::set_sizeDelta(text_tr, size_delta);

            let mut anchored_pos = RectTransform::get_anchoredPosition(text_tr);
            if let Some(x) = layout.anchored_pos_x {
                anchored_pos.x = x;
            }
            anchored_pos.y = layout.anchored_pos_y;
            RectTransform::set_anchoredPosition(text_tr, anchored_pos);
        }
    }
}

type Setup3Fn = extern "C" fn(this: *mut Il2CppObject, data: *mut Il2CppObject, succession_first: *mut Il2CppObject, succession_second: *mut Il2CppObject);
extern "C" fn Setup3(this: *mut Il2CppObject, data: *mut Il2CppObject, succession_first: *mut Il2CppObject, succession_second: *mut Il2CppObject) {
    get_orig_fn!(Setup3, Setup3Fn)(this, data, succession_first, succession_second);
    on_setup(this);
}

type Setup2Fn = extern "C" fn(this: *mut Il2CppObject, before: *mut Il2CppObject, after: *mut Il2CppObject);
extern "C" fn Setup2(this: *mut Il2CppObject, before: *mut Il2CppObject, after: *mut Il2CppObject) {
    get_orig_fn!(Setup2, Setup2Fn)(this, before, after);
    on_setup(this);
}

type BaseSetupFn = extern "C" fn(this: *mut Il2CppObject, data: *mut Il2CppObject);
extern "C" fn BaseSetup(this: *mut Il2CppObject, data: *mut Il2CppObject) {
    get_orig_fn!(BaseSetup, BaseSetupFn)(this, data);
    if !this.is_null() && unsafe { il2cpp_class_is_assignable_from(class(), (*this).klass()) } {
        on_setup(this);
    }
}

pub fn init(umamusume: *const Il2CppImage) {
    if Hachimi::instance().game.region != Region::Japan {
        return;
    }

    get_class_or_return!(umamusume, Gallop, PartsSingleModeStartProperFrame);
    get_class_or_return!(umamusume, Gallop, PartsProperFrame);

    unsafe {
        CLASS = PartsSingleModeStartProperFrame;
    }

    let BaseSetup_addr = get_method_addr(PartsProperFrame, c"Setup", 1);
    new_hook!(BaseSetup_addr, BaseSetup);

    let Setup3_addr = get_method_addr(PartsSingleModeStartProperFrame, c"Setup", 3);
    new_hook!(Setup3_addr, Setup3);

    let Setup2_addr = get_method_addr(PartsSingleModeStartProperFrame, c"Setup", 2);
    new_hook!(Setup2_addr, Setup2);
}
