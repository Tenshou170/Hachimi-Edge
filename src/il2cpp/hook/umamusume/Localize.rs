use std::{collections::BTreeMap, sync::RwLock};

use fnv::FnvHashMap;
use once_cell::sync::Lazy;

use crate::{
    core::{utils, Hachimi, SugoiClient},
    il2cpp::{ext::{Il2CppStringExt, StringExt}, symbols::{get_method_overload_addr, unbox}, types::*}
};

use super::TextId;

/// Maps TextId integer values to their enum name strings.
/// Populated lazily on first access per id. Using RwLock so concurrent reads
/// (which are the common case) don't contend with each other.
/// Previously this was `static mut LazyCell<HashMap>` which is unsound if
/// `Get` is ever called from more than one thread.
static TEXTID_NAME_CACHE: Lazy<RwLock<FnvHashMap<i32, String>>> =
    Lazy::new(|| RwLock::new(FnvHashMap::default()));

/**
 * Gallop::Localize::Get
 * Used by the game to get localized strings for builtin text (mostly UI).
 * 
 * id is a value of the TextId enum
 * cy devs likes to insert stuff at arbitrary locations within the enum, changing their values
 * so we'll just map them to their actual name instead
 */
type GetFn = extern "C" fn(id: i32) -> *mut Il2CppString;
pub extern "C" fn Get(id: i32) -> *mut Il2CppString {
    let hachimi = Hachimi::instance();
    let localized_data = hachimi.localized_data.load();
    if localized_data.localize_dict.is_empty() {
        return get_orig_fn!(Get, GetFn)(id);
    }

    // Fast path: read lock only
    let name_opt = TEXTID_NAME_CACHE.read().unwrap().get(&id).cloned();
    let name = if let Some(n) = name_opt {
        n
    } else {
        // Slow path: resolve and insert under write lock
        let name_ptr = TextId::get_name(id);
        let name_str = unsafe { (*name_ptr).as_utf16str().to_string() };
        TEXTID_NAME_CACHE.write().unwrap().insert(id, name_str.clone());
        name_str
    };

    let config = hachimi.config.load();
    if let Some(text) = localized_data.localize_dict.get(&name) {
        if config.text_debug && config.text_localize_dump {
            let orig_str = get_orig_fn!(Get, GetFn)(id);
            let orig_s = if orig_str.is_null() { String::new() } else { unsafe { (*orig_str).as_utf16str().to_string() } };
            info!("[Localize] key: {}, original: {}, localized: {}", name, orig_s, text);
        }
        text.to_il2cpp_string()
    }
    else {
        let str = get_orig_fn!(Get, GetFn)(id);
        if Hachimi::instance().config.load().translator_mode && id != 1109 && id != 1032 {
            // 1109 and 1032 seems to be debugging strings (they're annoying)
            utils::print_json_entry(&name, unsafe { &(*str).as_utf16str().to_string() });
        }
        if hachimi.config.load().auto_translate_localize && !str.is_null() && unsafe { (*str).length > 0 } {
            let s = unsafe { (*str).as_utf16str().to_string() };

            let sugoi = SugoiClient::instance();
            if let Some(translated) = sugoi.get_cached(&s) {
                return translated.to_il2cpp_string();
            } else {
                sugoi.translate_async(s);
            }
        }
        str
    }
}

pub fn dump_strings() -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();

    for obj in TextId::get_values().enumerator().map(|e| e.iter()).unwrap_or_default().expect("enum values enumerator") {
        let value: i32 = unsafe { unbox(obj) };
        let name = TextId::get_name(value);
        let name_str = unsafe { (*name).as_utf16str() };

        let res = get_orig_fn!(Get, GetFn)(value);
        if !res.is_null() {
            let res_str = unsafe { (*res).as_utf16str() };
            map.insert(name_str.to_string(), res_str.to_string());
        }
    }

    map
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, Localize);
    find_nested_class_or_return!(Localize, JP);

    let Get_addr = get_method_overload_addr(JP, "Get", &[Il2CppTypeEnum_IL2CPP_TYPE_VALUETYPE]);

    new_hook!(Get_addr, Get);
}
