use crate::{core::Hachimi, il2cpp::{symbols::{IEnumerator, MoveNextFn, SingletonLike, get_method_addr}, types::*}};
#[cfg(target_os = "windows")]
use crate::windows::free_camera::{self, CameraScene};
#[cfg(target_os = "windows")]
use crate::core::live_utils;
#[cfg(target_os = "windows")]
use std::sync::atomic::{AtomicBool, Ordering};
use super::Director;

static GAME_INITIALIZED: AtomicBool = AtomicBool::new(false);
pub fn is_initialized() -> bool {
    GAME_INITIALIZED.load(Ordering::Relaxed)
}

use super::GraphicSettings::{self, MsaaQuality};

static mut CLASS: *mut Il2CppClass = 0 as _;
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

pub fn instance() -> *mut Il2CppObject {
    let Some(singleton) = SingletonLike::new(class()) else {
        return 0 as _;
    };
    singleton.instance()
}

static mut SOFTWARERESET_ADDR: usize = 0;
impl_addr_wrapper_fn!(SoftwareReset, SOFTWARERESET_ADDR, (), this: *mut Il2CppObject);

type GameSystemUpdateFn = extern "C" fn(this: *mut Il2CppObject);
#[cfg(target_os = "windows")]
fn apply_free_camera_live_pause_request() {
    if !free_camera::take_toggle_live_pause_request() {
        return;
    }
    live_utils::toggle_live_pause();
}

extern "C" fn GameSystem_Update(this: *mut Il2CppObject) {
    crate::core::gui::race_slider_drain();
    // Refresh GUI-facing race state caches while we're on the il2cpp thread:
    // egui must not reach into il2cpp objects directly (kairusds #249 fix).
    super::RaceManagerReplayBase::refresh_playback_gated_cache();
    // catch_unwind, not a bare call: this is new, less-battle-tested code running every
    // frame on the main thread - a panic here must not be allowed to take the whole
    // process down with it (matches the defensive pattern the race-director-plugin this
    // was ported from used everywhere for the same reason).
    if std::panic::catch_unwind(super::race_telemetry::collect_frame).is_err() {
        error!("race_telemetry::collect_frame PANICKED (caught)");
    }

    #[cfg(target_os = "windows")]
    {
        apply_free_camera_live_pause_request();

        // Live and race normally tick from their camera LateUpdate hooks. Keep the
        // global update path only as a fallback while LiveTimelineControl is paused.
        if Director::is_live_paused() && free_camera::scene() == CameraScene::Live {
            free_camera::tick();
            apply_free_camera_live_pause_request();
        }
    }

    get_orig_fn!(GameSystem_Update, GameSystemUpdateFn)(this);
}

#[cfg(target_os = "windows")]
type GameSystemLateUpdateFn = extern "C" fn(this: *mut Il2CppObject);
#[cfg(target_os = "windows")]
extern "C" fn GameSystem_LateUpdate(this: *mut Il2CppObject) {
    get_orig_fn!(GameSystem_LateUpdate, GameSystemLateUpdateFn)(this);
    Director::apply_paused_free_camera();
}

// good hook for initializing values i guess
pub fn on_game_initialized() {
    GAME_INITIALIZED.store(true, Ordering::Relaxed);
    Hachimi::instance().init_character_data();
    #[cfg(target_os = "android")]
    crate::android::utils::set_audio_capture_policy_all();
    #[cfg(target_os = "windows")]
    super::UIManager::apply_ui_scale();

    if Hachimi::instance().config.load().msaa != MsaaQuality::Disabled {
        let graphic_settings = GraphicSettings::instance();
        if !graphic_settings.is_null() {
            GraphicSettings::set__isMSAA(graphic_settings, true);
        }
    }

    // Invoke plugin callbacks
    let hachimi = Hachimi::instance();
    let callbacks = hachimi.plugin_init_callbacks.lock().unwrap();
    for (callback, userdata) in callbacks.iter() {
        let callback_ptr = *callback;
        if callback_ptr == 0 { continue; }
        let callback: unsafe extern "C" fn(*mut std::ffi::c_void) = unsafe { std::mem::transmute(callback_ptr) };
        unsafe { callback(*userdata as *mut std::ffi::c_void); }
    }
}

extern "C" fn InitializeGame_MoveNext(enumerator: *mut Il2CppObject) -> bool {
    let moved = get_orig_fn!(InitializeGame_MoveNext, MoveNextFn)(enumerator);
    if !moved {
        // Game has finished initializing
        on_game_initialized();
    }
    moved
}

fn InitializeGameCommon(enumerator: IEnumerator) -> IEnumerator {
    if let Err(e) = enumerator.hook_move_next(InitializeGame_MoveNext) {
        error!("Failed to hook InitializeGame enumerator: {}", e);
    }

    enumerator
}

type InitializeGameJpFn = extern "C" fn(this: *mut Il2CppObject, on_complete_initialize_ui: *mut Il2CppObject) -> IEnumerator;
extern "C" fn InitializeGameJp(this: *mut Il2CppObject, on_complete_initialize_ui: *mut Il2CppObject) -> IEnumerator {
    let enumerator = get_orig_fn!(InitializeGameJp, InitializeGameJpFn)(this, on_complete_initialize_ui);
    InitializeGameCommon(enumerator)
}

type InitializeGameOtherFn = extern "C" fn(this: *mut Il2CppObject) -> IEnumerator;
extern "C" fn InitializeGameOther(this: *mut Il2CppObject) -> IEnumerator {
    let enumerator = get_orig_fn!(InitializeGameOther, InitializeGameOtherFn)(this);
    InitializeGameCommon(enumerator)
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, GameSystem);

    let InitializeGame1_addr = get_method_addr(GameSystem, c"InitializeGame", 1);
    if InitializeGame1_addr != 0 {
        new_hook!(InitializeGame1_addr, InitializeGameJp);
    } else {
        let InitializeGame0_addr = get_method_addr(GameSystem, c"InitializeGame", 0);
        if InitializeGame0_addr != 0 {
            new_hook!(InitializeGame0_addr, InitializeGameOther);
        }
    }

    unsafe {
        CLASS = GameSystem;
        SOFTWARERESET_ADDR = get_method_addr(GameSystem, c"SoftwareReset", 0);
    }

    let GameSystem_Update_addr = get_method_addr(GameSystem, c"Update", 0);
    new_hook!(GameSystem_Update_addr, GameSystem_Update);

    #[cfg(target_os = "windows")]
    {
        let GameSystem_LateUpdate_addr = get_method_addr(GameSystem, c"LateUpdate", 0);
        new_hook!(GameSystem_LateUpdate_addr, GameSystem_LateUpdate);
    }
}