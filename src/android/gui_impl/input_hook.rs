use std::sync::atomic::{AtomicBool, AtomicI32, AtomicI64, AtomicUsize, Ordering};
use once_cell::sync::OnceCell;

use egui::Vec2;
use jni::{
    objects::{JObject, GlobalRef},
    sys::{jboolean, jint, JNI_TRUE},
    JNIEnv,
};

use crate::{
    android::utils::{BACK_BUTTON_PRESSED, IS_IME_VISIBLE, get_activity, get_screen_dimensions},
    core::{Error, Gui, Hachimi},
    il2cpp::symbols::Thread
};

static KEY_EVENT_CLASS: OnceCell<GlobalRef> = OnceCell::new();
static MOTION_EVENT_CLASS: OnceCell<GlobalRef> = OnceCell::new();

use super::keymap;

const ACTION_DOWN: jint = 0;
const ACTION_UP: jint = 1;
const ACTION_MOVE: jint = 2;
const ACTION_POINTER_DOWN: jint = 5;
const ACTION_POINTER_UP: jint = 6;
const ACTION_HOVER_MOVE: jint = 7;
const ACTION_SCROLL: jint = 8;
const ACTION_MASK: jint = 0xff;
const ACTION_POINTER_INDEX_MASK: jint = 0xff00;
const ACTION_POINTER_INDEX_SHIFT: jint = 8;

const TOOL_TYPE_MOUSE: jint = 3;

const AXIS_VSCROLL: jint = 9;
const AXIS_HSCROLL: jint = 10;
static SCROLL_AXIS_SCALE: f32 = 10.0;

static VOLUME_UP_PRESSED: AtomicBool = AtomicBool::new(false);
static VOLUME_DOWN_PRESSED: AtomicBool = AtomicBool::new(false);
static POINTER_CAPTURED: AtomicBool = AtomicBool::new(false);

pub struct MultiTapState {
    pub count: AtomicUsize,
    pub last_tap_time: AtomicI64,
}

impl MultiTapState {
    pub const fn new() -> Self {
        Self {
            count: AtomicUsize::new(0),
            last_tap_time: AtomicI64::new(0),
        }
    }

    pub fn register_tap(&self, limit: usize, window_ms: i64) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        let last_time = self.last_tap_time.swap(now, Ordering::Relaxed); // Update time immediately and get old
        let delta = now - last_time;

        if delta > window_ms || last_time == 0 {
            self.count.store(1, Ordering::Relaxed);
            return limit == 1;
        }

        let current = self.count.fetch_add(1, Ordering::Relaxed) + 1;

        if current >= limit {
            self.count.store(0, Ordering::Relaxed);
            self.last_tap_time.store(0, Ordering::Relaxed);
            return true;
        }

        false
    }
}

const CORNER_TAP_LIMIT: usize = 3;
const VOLUME_TAP_LIMIT: usize = 2;
const RESET_GUI_CONSUMING_TAP_LIMIT: usize = 4;
const TAP_WINDOW_MS: i64 = 300;
const CORNER_ZONE_RATIO: f32 = 0.12; // 12% screen size

static VOLUME_UP_STATE: MultiTapState = MultiTapState::new();
static CORNER_TAP_STATE: MultiTapState = MultiTapState::new();
static TOGGLE_GAME_UI_TAP_STATE: MultiTapState = MultiTapState::new();
static RESET_GUI_CONSUMING_STATE: MultiTapState = MultiTapState::new();
static SCREEN_WIDTH: AtomicI32 = AtomicI32::new(0);
static SCREEN_HEIGHT: AtomicI32 = AtomicI32::new(0);

type NativeInjectEventFn = extern "C" fn(env: JNIEnv, obj: JObject, input_event: JObject, extra_param: jint) -> jboolean;
extern "C" fn nativeInjectEvent(mut env: JNIEnv, obj: JObject, input_event: JObject, extra_param: jint) -> jboolean {
    if input_event.is_null() {
        return 0;
    }

    let handle_event = |env: &mut JNIEnv| -> jni::errors::Result<Option<jboolean>> {
        let action = env.call_method(&input_event, "getAction", "()I", &[])?.i()?;
        let action_masked = action & ACTION_MASK;
        let is_consuming = Gui::is_consuming_input_atomic();

        if !is_consuming && (action_masked == ACTION_MOVE || action_masked == ACTION_HOVER_MOVE) {
            return Ok(None); // forward
        }

        let key_event_class = KEY_EVENT_CLASS.get_or_try_init(|| {
            let local_class = env.find_class("android/view/KeyEvent")?;
            env.new_global_ref(local_class)
        })?;
        if env.is_instance_of(&input_event, key_event_class)? {
            let key_code = env.call_method(&input_event, "getKeyCode", "()I", &[])?.i()?;
            let repeat_count = env.call_method(&input_event, "getRepeatCount", "()I", &[])?.i()?;

            let pressed = action == ACTION_DOWN;

            match key_code {
                keymap::KEYCODE_VOLUME_UP => {
                    VOLUME_UP_PRESSED.store(pressed, Ordering::Relaxed);

                    if pressed && repeat_count == 0
                        && VOLUME_UP_STATE.register_tap(VOLUME_TAP_LIMIT, TAP_WINDOW_MS)
                        && Hachimi::instance().config.load().hide_ingame_ui_hotkey {
                        Thread::main_thread().schedule(Gui::toggle_game_ui);
                        return Ok(Some(JNI_TRUE));
                    }

                    if pressed && VOLUME_DOWN_PRESSED.load(Ordering::Relaxed) {
                        if let Some(mut gui) = Gui::instance().map(|m| m.lock().unwrap()) {
                            gui.toggle_menu();
                        }
                    }
                }

                keymap::KEYCODE_VOLUME_DOWN => {
                    VOLUME_DOWN_PRESSED.store(pressed, Ordering::Relaxed);

                    if pressed && VOLUME_UP_PRESSED.load(Ordering::Relaxed) && repeat_count == 0 {
                        if let Some(mut gui) = Gui::instance().map(|m| m.lock().unwrap()) {
                            gui.toggle_menu();
                        }
                    }
                    
                    if pressed && RESET_GUI_CONSUMING_STATE.register_tap(RESET_GUI_CONSUMING_TAP_LIMIT, TAP_WINDOW_MS) {
                        if let Some(mut gui) = Gui::instance().map(|m| m.lock().unwrap()) {
                            gui.set_consuming_input(false);
                        }
                        return Ok(Some(JNI_TRUE));
                    }
                }

                _ => {
                    // Generic keybind capture — used by SetKeybindWindow for rebinding.
                    if crate::core::gui::is_keybind_capture_active() && pressed && repeat_count == 0 {
                        let display = keymap::keycode_display_label(key_code);
                        crate::core::gui::report_keybind_capture(key_code, display);
                        return Ok(Some(JNI_TRUE));
                    }

                    if pressed && key_code == Hachimi::instance().config.load().android.menu_open_key {
                        let Some(mut gui) = Gui::instance().map(|m| m.lock().unwrap()) else {
                            return Ok(None); // forward
                        };
                        gui.toggle_menu();
                    }

                    if Hachimi::instance().config.load().hide_ingame_ui_hotkey && pressed
                        && key_code == Hachimi::instance().config.load().android.hide_ingame_ui_hotkey_bind {
                        Thread::main_thread().schedule(Gui::toggle_game_ui);
                    }

                    if pressed && key_code == keymap::KEYCODE_BACK {
                        BACK_BUTTON_PRESSED.store(pressed, Ordering::Release);
                        if IS_IME_VISIBLE.load(Ordering::Acquire) {
                            return Ok(Some(JNI_TRUE)); 
                        }
                    }

                    if Gui::is_consuming_input_atomic() {
                        {
                            let Some(mut gui) = Gui::instance().map(|m| m.lock().unwrap()) else {
                                return Ok(None); // forward
                            };

                            if let Some(key) = keymap::get_key(key_code) {
                                gui.input.events.push(egui::Event::Key {
                                    key,
                                    physical_key: None,
                                    pressed,
                                    repeat: false,
                                    modifiers: Default::default()
                                });
                            }

                            if pressed {
                                let c = env.call_method(&input_event, "getUnicodeChar", "()I", &[])?.i()?;
                                if c != 0 {
                                    if let Some(c) = char::from_u32(c as _) {
                                        gui.input.events.push(egui::Event::Text(c.to_string()));
                                    }
                                }
                            }
                        }

                        if !Gui::wants_input_atomic() {
                            return Ok(None); // forward
                        }
                        return Ok(Some(JNI_TRUE));
                    }
                }
            }

            return Ok(None); // forward
        }

        let motion_event_class = MOTION_EVENT_CLASS.get_or_try_init(|| {
            let local_class = env.find_class("android/view/MotionEvent")?;
            env.new_global_ref(local_class)
        })?;
        if env.is_instance_of(&input_event, motion_event_class)? {
            let pointer_index = (action & ACTION_POINTER_INDEX_MASK) >> ACTION_POINTER_INDEX_SHIFT;

            let real_x = env.call_method(&input_event, "getX", "()F", &[])?.f()?;
            let real_y = env.call_method(&input_event, "getY", "()F", &[])?.f()?;

            if action_masked == ACTION_DOWN {
                let mut current_w = SCREEN_WIDTH.load(Ordering::Relaxed);
                let mut current_h = SCREEN_HEIGHT.load(Ordering::Relaxed);

                let mut corner_zone_size = if current_w > 0 && current_h > 0 {
                    (current_w.min(current_h) as f32) * CORNER_ZONE_RATIO
                } else {
                    150.0
                };

                let out_of_bounds = real_x > current_w as f32 || real_y > current_h as f32;
                let is_bottom_left_rotation = current_h > current_w && real_x < corner_zone_size && real_y < (current_h as f32 * 0.6);
                let looks_wrong = is_bottom_left_rotation || (current_w > current_h && real_y < corner_zone_size && real_x < (current_w as f32 * 0.6));

                if current_h == 0 || out_of_bounds || looks_wrong {
                    let (new_w, new_h) = get_screen_dimensions(unsafe { env.unsafe_clone() });
                    SCREEN_WIDTH.store(new_w, Ordering::Relaxed);
                    SCREEN_HEIGHT.store(new_h, Ordering::Relaxed);
                    current_w = new_w;
                    current_h = new_h;

                    if current_w > 0 && current_h > 0 {
                        corner_zone_size = (current_w.min(current_h) as f32) * CORNER_ZONE_RATIO;
                    }
                }

                // top left (toggle gui)
                if !Hachimi::instance().config.load().disable_gui
                    && real_x < corner_zone_size && real_y < corner_zone_size
                    && CORNER_TAP_STATE.register_tap(CORNER_TAP_LIMIT, TAP_WINDOW_MS) {
                    let Some(mut gui) = Gui::instance().map(|m| m.lock().unwrap()) else {
                        return Ok(None); // forward
                    };
                    gui.toggle_menu();
                    return Ok(Some(JNI_TRUE));
                }

                // top right (toggle in-game ui)
                if Hachimi::instance().config.load().hide_ingame_ui_hotkey
                    && real_x > (current_w as f32 - corner_zone_size) && real_y < corner_zone_size
                    && TOGGLE_GAME_UI_TAP_STATE.register_tap(CORNER_TAP_LIMIT, TAP_WINDOW_MS) {
                    Thread::main_thread().schedule(Gui::toggle_game_ui);
                    return Ok(Some(JNI_TRUE));
                }
            }

            if !is_consuming {
                return Ok(None); // forward
            }

            if pointer_index != 0 {
                return Ok(None); // forward
            }

            let mut capture = false;

            {
                let Some(mut gui) = Gui::instance().map(|m| m.lock().unwrap()) else {
                    return Ok(None); // forward
                };

                let ppp = get_ppp(unsafe { env.unsafe_clone() }, &gui);
                let pos = egui::Pos2 { x: real_x / ppp, y: real_y / ppp };

                match action_masked {
                    ACTION_DOWN | ACTION_POINTER_DOWN | ACTION_SCROLL => {
                        if let Some(layer) = gui.context.layer_id_at(pos) {
                            if layer.order != egui::Order::Background {
                                capture = true;
                            }
                        }
                        if !capture && gui.is_consuming_input() {
                            let menu_w = crate::core::gui::get_menu_width();
                            if pos.x < menu_w {
                                capture = true;
                            }
                        }
                        POINTER_CAPTURED.store(capture, Ordering::Release);
                    }
                    ACTION_MOVE | ACTION_HOVER_MOVE => {
                        capture = POINTER_CAPTURED.load(Ordering::Acquire);
                    }
                    ACTION_UP | ACTION_POINTER_UP => {
                        capture = POINTER_CAPTURED.load(Ordering::Acquire);
                        POINTER_CAPTURED.store(false, Ordering::Release);
                    }
                    _ => return Ok(Some(JNI_TRUE))
                }

                if action_masked == ACTION_SCROLL {
                    let x = env.call_method(&input_event, "getAxisValue", "(I)F", &[AXIS_HSCROLL.into()])?.f()?;
                    let y = env.call_method(&input_event, "getAxisValue", "(I)F", &[AXIS_VSCROLL.into()])?.f()?;
                    gui.input.events.push(egui::Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Point,
                        delta: Vec2::new(x, y) * SCROLL_AXIS_SCALE,
                        modifiers: egui::Modifiers::default(),
                    });
                }
                else {
                    let phase = match action_masked {
                        ACTION_DOWN | ACTION_POINTER_DOWN => egui::TouchPhase::Start,
                        ACTION_MOVE | ACTION_HOVER_MOVE => egui::TouchPhase::Move,
                        ACTION_UP | ACTION_POINTER_UP => egui::TouchPhase::End,
                        _ => return Ok(Some(JNI_TRUE))
                    };

                    let tool_type = env.call_method(&input_event, "getToolType", "(I)I", &[0.into()])?.i()?;

                    match phase {
                        egui::TouchPhase::Start => {
                            gui.input.events.push(egui::Event::PointerMoved(pos));
                            gui.input.events.push(egui::Event::PointerButton {
                                pos,
                                button: egui::PointerButton::Primary,
                                pressed: true,
                                modifiers: Default::default()
                            });
                        },
                        egui::TouchPhase::Move => {
                            gui.input.events.push(egui::Event::PointerMoved(pos));
                        },
                        egui::TouchPhase::End | egui::TouchPhase::Cancel => {
                            gui.input.events.push(egui::Event::PointerButton {
                                pos,
                                button: egui::PointerButton::Primary,
                                pressed: false,
                                modifiers: Default::default()
                            });
                            if tool_type != TOOL_TYPE_MOUSE {
                                gui.input.events.push(egui::Event::PointerGone);
                            }
                        }
                    }
                }
            }

            if !capture {
                return Ok(None); // forward
            }

            return Ok(Some(JNI_TRUE));
        }

        Ok(None) // forward
    };

    match handle_event(&mut env) {
        Ok(Some(res)) => res,
        Ok(None) => get_orig_fn!(nativeInjectEvent, NativeInjectEventFn)(env, obj, input_event, extra_param),
        Err(_) => {
            if env.exception_check().unwrap_or(false) {
                let _ = env.exception_clear();
            }
            get_orig_fn!(nativeInjectEvent, NativeInjectEventFn)(env, obj, input_event, extra_param)
        }
    }
}

fn get_ppp(mut env: JNIEnv, gui: &Gui) -> f32 {
    let Some(view) = get_view(unsafe { env.unsafe_clone() }) else {
        return gui.context.pixels_per_point();
    };

    let result = (|| -> jni::errors::Result<f32> {
        let view_width = env.call_method(&view, "getWidth", "()I", &[])?.i()?;
        let view_height = env.call_method(&view, "getHeight", "()I", &[])?.i()?;
        let view_main_axis_size = if view_width < view_height { view_width } else { view_height };
        Ok(gui.context.zoom_factor() * (view_main_axis_size as f32 / gui.prev_main_axis_size as f32))
    })();

    match result {
        Ok(val) => val,
        Err(_) => {
            if env.exception_check().unwrap_or(false) {
                let _ = env.exception_clear();
            }
            gui.context.pixels_per_point()
        }
    }
}

fn get_view(mut env: JNIEnv<'_>) -> Option<JObject<'_>> {
    let activity = get_activity(unsafe { env.unsafe_clone() })?;
    let jni_window = env
        .call_method(activity, "getWindow", "()Landroid/view/Window;", &[])
        .ok()?
        .l()
        .ok()?;

    env.call_method(jni_window, "getDecorView", "()Landroid/view/View;", &[])
        .ok()?
        .l()
        .ok()
}

pub static mut NATIVE_INJECT_EVENT_ADDR: usize = 0;

fn init_internal() -> Result<(), Error> {
    let native_inject_event_addr = unsafe { NATIVE_INJECT_EVENT_ADDR };
    if native_inject_event_addr != 0 {
        info!("Hooking nativeInjectEvent");
        Hachimi::instance().interceptor.hook(unsafe { NATIVE_INJECT_EVENT_ADDR }, nativeInjectEvent as usize)?;
    }
    else {
        error!("native_inject_event_addr is null");
    }

    Ok(())
}

pub fn init() {
    init_internal().unwrap_or_else(|e| {
        error!("Init failed: {}", e);
    });
}
