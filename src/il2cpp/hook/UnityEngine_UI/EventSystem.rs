#[cfg(target_os = "windows")]
use std::sync::atomic::{AtomicBool, Ordering};
use crate::il2cpp::{symbols::{get_method_addr}, types::*};

static mut GET_CURRENT_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_current, GET_CURRENT_ADDR, *mut Il2CppObject,);

static mut GET_CURRENTSELECTEDGAMEOBJECT_ADDR: usize = 0;
impl_addr_wrapper_fn!(get_currentSelectedGameObject, GET_CURRENTSELECTEDGAMEOBJECT_ADDR, *mut Il2CppObject, this: *mut Il2CppObject);

/// Cached at first Update call — avoids re-checking ntdll every frame.
#[cfg(target_os = "windows")]
static IS_WINE: AtomicBool = AtomicBool::new(false);
#[cfg(target_os = "windows")]
static IS_WINE_CHECKED: AtomicBool = AtomicBool::new(false);

#[cfg(target_os = "windows")]
fn smtc_on_update_if_native() {
    if !IS_WINE_CHECKED.load(Ordering::Relaxed) {
        IS_WINE.store(crate::windows::hachimi_impl::is_wine(), Ordering::Relaxed);
        IS_WINE_CHECKED.store(true, Ordering::Relaxed);
    }
    if !IS_WINE.load(Ordering::Relaxed) {
        crate::windows::smtc::on_update();
    }
}

type UpdateFn = extern "C" fn(this: *mut Il2CppObject);
extern "C" fn Update(this: *mut Il2CppObject) {
    get_orig_fn!(Update, UpdateFn)(this);

    let mut completed = Vec::new();
    {
        let rx = crate::core::sugoi_client::TRANSLATION_QUEUE.1.lock().unwrap();
        while let Ok(msg) = rx.try_recv() {
            completed.push(msg);
        }
    }

    crate::il2cpp::hook::UnityEngine_UI::Text::prune_inactive_translation_targets();
    crate::il2cpp::hook::UnityEngine_TextRenderingModule::TextMesh::prune_inactive_translation_targets();

    if completed.is_empty() {
        #[cfg(target_os = "windows")]
        smtc_on_update_if_native();
        return;
    }

    {
        let mut cache = crate::core::sugoi_client::TRANSLATION_CACHE.lock().unwrap();
        for (orig, trans) in &completed {
            cache.put(orig.clone(), trans.clone());
        }
    }

    crate::il2cpp::hook::UnityEngine_UI::Text::apply_translations(&completed);
    crate::il2cpp::hook::UnityEngine_TextRenderingModule::TextMesh::apply_translations(&completed);

    #[cfg(target_os = "windows")]
    smtc_on_update_if_native();
}

pub fn init(UnityEngine_UI: *const Il2CppImage) {
    get_class_or_return!(UnityEngine_UI, "UnityEngine.EventSystems", EventSystem);

    let Update_addr = get_method_addr(EventSystem, c"Update", 0);
    new_hook!(Update_addr, Update);

    unsafe {
        GET_CURRENT_ADDR = get_method_addr(EventSystem, c"get_current", 0);
        GET_CURRENTSELECTEDGAMEOBJECT_ADDR = get_method_addr(EventSystem, c"get_currentSelectedGameObject", 0);
    }
}
