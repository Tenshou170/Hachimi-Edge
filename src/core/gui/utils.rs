use egui_material3::*;
use rust_i18n::t;
use super::*;
use std::sync::Arc;
use std::time::Instant;

pub enum KeyboardOwner {
    JNI(egui::Id),
    Unity(egui::Id),
}

pub fn get_scale(ctx: &egui::Context) -> f32 {
    ctx.data(|d| d.get_temp::<f32>(egui::Id::new("gui_scale")))
        .unwrap_or(1.0)
}

pub fn get_scale_salt(ctx: &egui::Context) -> f32 {
    ctx.data(|d| d.get_temp::<f32>(egui::Id::new("gui_scale_salt")))
        .unwrap_or(1.0)
}

#[cfg(target_os = "android")]
pub fn is_ime_visible() -> bool {
    let kb_ptr = ACTIVE_KEYBOARD.load(Ordering::Acquire);
    let unity_visible = if !kb_ptr.is_null() {
        TouchScreenKeyboard::get_status(kb_ptr) == TouchScreenKeyboard::Status::Visible
    } else {
        false
    };
    let jni_visible = crate::android::utils::IS_IME_VISIBLE.load(Ordering::Acquire);

    unity_visible || jni_visible
}

#[cfg(target_os = "android")]
pub fn ime_scroll_padding(ctx: &egui::Context) -> f32 {
    if !is_ime_visible() {
        return 0.0;
    }
    ctx.input(|i| i.viewport_rect().height() * 0.35)
}

/// Non-Android stub — IME padding is always zero on platforms without a
/// software keyboard managed by the Unity TouchScreenKeyboard API.
#[cfg(not(target_os = "android"))]
pub fn ime_scroll_padding(_ctx: &egui::Context) -> f32 {
    0.0
}

#[derive(Default)]
pub struct LiveSliderCache {
    pub director_class: usize,
    pub director_instance_field: usize,
    pub get_current_time: usize,
    pub get_total_time: usize,
    pub is_pause_live: usize,
}

#[cfg(target_os = "windows")]
pub fn wine_unavailable_hint(ui: &mut egui::Ui) {
    // Use onSurfaceVariant at full opacity — it's already muted enough to read
    // as secondary text, but light enough to be legible on dark backgrounds.
    let color = egui_material3::theme::get_global_color("onSurfaceVariant");
    ui.label(
        egui::RichText::new(t!("config_editor.unavailable_wine_proton"))
            .size(12.0)
            .color(color)
    );
}

#[cfg(target_os = "windows")]
pub fn wine_unavailable_setting_label(ui: &mut egui::Ui, label: impl Into<egui::WidgetText>) {
    ui.vertical(|ui| {
        ui.label(label);
        wine_unavailable_hint(ui);
    });
}

pub struct TweenInOutWithDelay {
    tween_time: f32,
    delay_duration: f32,
    easing: Easing,

    started: bool,
    delay_start: Option<Instant>,
}

impl TweenInOutWithDelay {
    pub fn new(tween_time: f32, delay_duration: f32, easing: Easing) -> TweenInOutWithDelay {
        TweenInOutWithDelay {
            tween_time,
            delay_duration,
            easing,

            started: false,
            delay_start: None,
        }
    }

    pub fn run(&mut self, ctx: &egui::Context, id: egui::Id) -> Option<f32> {
        let anim_dir = if let Some(start) = self.delay_start {
            // Hold animation at peak position until duration passes
            start.elapsed().as_secs_f32() < self.delay_duration
        } else {
            // On animation start, initialize to 0.0. Next calls will start tweening to 1.0
            let v = self.started;
            self.started = true;
            v
        };
        let tween_val = ctx.animate_bool_with_time(id, anim_dir, self.tween_time);

        // Switch on delay when animation hits peak (next call makes tween_val < 1.0)
        if tween_val == 1.0 && self.delay_start.is_none() {
            self.delay_start = Some(Instant::now());
        }
        // Check if everything's done
        else if tween_val == 0.0 && self.delay_start.is_some() {
            return None;
        }

        Some(match self.easing {
            //Easing::Linear => tween_val,
            //Easing::InQuad => tween_val * tween_val,
            Easing::OutQuad => 1.0 - (1.0 - tween_val) * (1.0 - tween_val),
        })
    }
}

pub enum Easing {
    //Linear,
    //InQuad,
    OutQuad,
}

pub struct NotificationGuard(pub u32);

impl Drop for NotificationGuard {
    fn drop(&mut self) {
        if let Some(mutex) = Gui::instance() {
            if let Ok(mut gui) = mutex.lock() {
                gui.close_notification(self.0);
            }
        }
    }
}

pub fn random_id() -> egui::Id {
    egui::Id::new(egui::epaint::ahash::RandomState::new().hash_one(0))
}

/// Returns the width of the control column (column 2) in the config grid,
/// as published by the grid setup in ConfigEditor::run().
/// Falls back to ui.available_width() when called outside the grid context.
pub fn grid_control_w(ui: &egui::Ui) -> f32 {
    ui.data(|d| d.get_temp::<f32>(egui::Id::new("grid_control_w")))
        .unwrap_or_else(|| ui.available_width())
}

/// Returns the standard window size for modal dialogs: 82% of viewport width
/// capped at 320 logical units, 62% of viewport height capped at 250.
/// Used by `new_window` defaults and other modal windows (theme editor, etc.).
pub fn standard_window_size(ctx: &egui::Context) -> egui::Vec2 {
    let scale = get_scale(ctx);
    let vp = ctx.viewport_rect();
    egui::vec2(
        (vp.width() * 0.82).min(320.0 * scale),
        (vp.height() * 0.62).min(250.0 * scale),
    )
}

/// Returns true when the viewport is taller than it is wide (portrait orientation).
/// On Windows we always use landscape layout since the game window can be any
/// size and portrait mode is not the primary use case for PC.
pub fn is_portrait(ctx: &egui::Context) -> bool {
    let vp = ctx.viewport_rect();
    vp.height() > vp.width()
}

/// Returns the window size for the ConfigEditor, adapted to orientation.
///
/// - **Portrait** (Android phone): nearly full-screen — 96% × 92%.
///   The nav rail (80dp) + content column need every pixel of width, and the
///   settings list can be long so we maximise height.
/// - **Landscape** (Windows game overlay / Android landscape): wide enough that
///   the per-row label+control layout never clips — 92% × 88%, capped at
///   640×420 dp so it doesn't completely obscure the game window.
pub fn config_editor_window_size(ctx: &egui::Context) -> egui::Vec2 {
    let scale = get_scale(ctx);
    let vp = ctx.viewport_rect();
    if is_portrait(ctx) {
        egui::vec2(
            (vp.width()  * 0.96).min(320.0 * scale),
            (vp.height() * 0.92).min(400.0 * scale),
        )
    } else {
        egui::vec2(
            (vp.width()  * 0.92).min(540.0 * scale),
            (vp.height() * 0.88).min(380.0 * scale),
        )
    }
}

pub fn new_window<'a>(
    ctx: &egui::Context,
    id: egui::Id,
    title: impl Into<egui::WidgetText>,
) -> egui::Window<'a> {
    let scale = get_scale(ctx);
    let salt = get_scale_salt(ctx);
    let vp = ctx.viewport_rect();
    let size = standard_window_size(ctx);

    // Unless the user explicitly enabled translucent windows in Theme Settings,
    // force all child windows to be fully opaque (the sidebar panel is unaffected).
    let mut frame = egui::Frame::window(&ctx.style());
    if !Hachimi::instance().config.load().ui_translucent_windows {
        frame.fill = egui::Color32::from_rgba_unmultiplied(
            frame.fill.r(), frame.fill.g(), frame.fill.b(), 255,
        );
    }

    egui::Window::new(title)
        .id(id.with(salt.to_bits()))
        .pivot(egui::Align2::CENTER_CENTER)
        // vp.center() is the true midpoint; vp.max is the bottom-right corner.
        .fixed_pos(vp.center())
        .min_width(96.0 * scale)
        .max_width(size.x)
        .max_height(size.y)
        .collapsible(false)
        .resizable(false)
        .default_size(size)
        .frame(frame)
}

pub fn simple_window_layout(
    ui: &mut egui::Ui,
    id: egui::Id,
    add_contents: impl FnOnce(&mut egui::Ui),
    add_buttons: impl FnOnce(&mut egui::Ui),
) {
    let builder = egui::UiBuilder::new()
        .id(id)
        .layout(egui::Layout::top_down(egui::Align::Center).with_cross_justify(true));

    ui.scope_builder(builder, |ui| {
        // cross_justify must stay active for the content area so that
        // full-width widgets (tab bar, grid) stretch edge-to-edge.
        ui.with_layout(
            egui::Layout::top_down(egui::Align::Min).with_cross_justify(true),
            add_contents,
        );

        ui.separator();

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), add_buttons);
    });
}

#[allow(dead_code)]
pub fn centered_and_wrapped_text(ui: &mut egui::Ui, text: &str) {
    let rect = ui.available_rect_before_wrap();

    let text_style = egui::TextStyle::Body;
    let text_font = ui
        .style()
        .text_styles
        .get(&text_style)
        .cloned()
        .unwrap_or_default();
    let text_color = ui.style().visuals.text_color();

    let mut job =
        egui::text::LayoutJob::simple(text.to_owned(), text_font, text_color, rect.width());
    job.halign = egui::Align::Center;

    let galley = ui.painter().layout_job(job);

    let text_rect = galley.rect;
    let text_size = text_rect.size();

    let center_pos = rect.min + (rect.size() - text_size) / 2.0;

    let paint_pos = center_pos - text_rect.min.to_vec2();
    ui.painter().galley(paint_pos, galley, text_color);
}

pub fn paginated_window_layout(
    ui: &mut egui::Ui,
    id: egui::Id,
    i: &mut usize,
    page_count: usize,
    allow_next: bool,
    add_page_content: impl FnOnce(&mut egui::Ui, usize),
) -> bool {
    let mut open = true;

    let builder = egui::UiBuilder::new()
        .id(id)
        .layout(egui::Layout::top_down(egui::Align::Center).with_cross_justify(true));

    ui.scope_builder(builder, |ui| {
        ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
            add_page_content(ui, *i);
        });

        ui.separator();

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            if *i < page_count - 1 {
                if allow_next && ui.add(MaterialButton::filled(t!("next"))).clicked() {
                    *i += 1;
                }
            } else {
                if ui.add(MaterialButton::filled(t!("done"))).clicked() {
                    open = false;
                }
            }
            if *i > 0 && ui.add(MaterialButton::text(t!("previous"))).clicked() {
                *i -= 1;
            }
        });
    });

    open
}

pub fn async_request_ui_content<T: Send + Sync + 'static>(
    ui: &mut egui::Ui,
    request: Arc<AsyncRequest<T>>,
    on_retry: impl FnOnce(),
    add_contents: impl FnOnce(&mut egui::Ui, &T),
) {
    let Some(result) = &**request.result.load() else {
        if !request.running() {
            request.call();
        }
        ui.centered_and_justified(|ui| {
            ui.label(t!("loading_label"));
        });
        return;
    };

    match result {
        Ok(v) => add_contents(ui, v),
        Err(e) => {
            let rect = ui.available_rect_before_wrap();

            let text_style = egui::TextStyle::Body;
            let text_font = ui
                .style()
                .text_styles
                .get(&text_style)
                .cloned()
                .unwrap_or_default();
            let text_color = ui.visuals().text_color();

            let mut text_job =
                egui::text::LayoutJob::simple(e.to_string(), text_font, text_color, rect.width());
            text_job.halign = egui::Align::Center;
            let text_galley = ui.painter().layout_job(text_job.clone());
            let text_height = text_galley.size().y;

            let btn_text = t!("retry");
            let btn_style = egui::TextStyle::Button;
            let btn_font = ui
                .style()
                .text_styles
                .get(&btn_style)
                .cloned()
                .unwrap_or_default();
            let btn_job = egui::text::LayoutJob::simple(
                btn_text.to_string(),
                btn_font,
                text_color,
                f32::INFINITY,
            );
            let btn_galley = ui.painter().layout_job(btn_job);
            let btn_padding = ui.style().spacing.button_padding;
            let btn_height = btn_galley.size().y + btn_padding.y * 2.0;

            let spacing = ui.spacing().item_spacing.y;
            let total_height = text_height + spacing + btn_height;

            let center_y = rect.center().y;
            let top_y = (center_y - total_height / 2.0).max(rect.top());

            let content_rect = egui::Rect::from_min_size(
                egui::pos2(rect.left(), top_y),
                egui::vec2(rect.width(), total_height),
            );

            let builder = egui::UiBuilder::new().max_rect(content_rect);
            ui.scope_builder(builder, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(text_job);
                    if ui.add(MaterialButton::filled(btn_text)).clicked() {
                        on_retry();
                    }
                });
            });
        }
    }
}

pub const STANDARD_SLIDER_WIDTH: f32 = 140.0;

/// Minimum row height for a settings list-tile row.
/// 36dp gives comfortable padding around a 28dp switch without the
/// wasted space that the full MD3 48dp touch-target spec adds in a
/// dense settings list context.
pub const LIST_TILE_H: f32 = 36.0;

/// Horizontal padding inside each list-tile row and the settings card.
pub const LIST_TILE_PAD_H: f32 = 16.0;

/// Vertical spacing between list-tile rows inside a card.
pub const LIST_TILE_SPACING: f32 = 0.0;

/// Renders an MD3-styled section heading using the `primary` color token.
///
/// Used above `settings_card` groups. Draws the label in primary color with
/// a `12sp` bold-weight font and a thin `outlineVariant` divider underneath.
pub fn section_heading(ui: &mut egui::Ui, text: impl Into<String>) {
    let primary        = egui_material3::theme::get_global_color("primary");
    let outline_variant = egui_material3::theme::get_global_color("outlineVariant");
    let text = text.into();

    let galley = ui.painter().layout_no_wrap(
        text,
        egui::FontId::proportional(14.0),  // larger — more readable as a section header
        primary,
    );

    let (resp, painter) = ui.allocate_painter(
        egui::vec2(ui.available_width(), galley.size().y + 8.0),
        egui::Sense::hover(),
    );
    let rect = resp.rect;

    // Text left-aligned, vertically centered.
    painter.galley(
        egui::pos2(rect.min.x + LIST_TILE_PAD_H, rect.min.y + (rect.height() - galley.size().y) / 2.0),
        galley,
        primary,
    );

    // Thin bottom divider.
    painter.line_segment(
        [rect.left_bottom(), rect.right_bottom()],
        egui::Stroke::new(1.0, outline_variant),
    );
}

/// Wraps `add_contents` in an MD3 "settings card" — a rounded rect with
/// `surfaceContainerLow` background and `12dp` corner radius.
///
/// Use this to group logically related settings rows, matching the Jetpack
/// Compose `Card` pattern used in modern Android settings screens.
pub fn settings_card(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    let surface_low     = egui_material3::theme::get_global_color("surfaceContainerLow");
    let outline_variant = egui_material3::theme::get_global_color("outlineVariant");
    let cr = egui_material3::theme::get_global_corner_radius();

    egui::Frame::NONE
        .fill(surface_low)
        .stroke(egui::Stroke::new(1.0, outline_variant))
        .corner_radius(egui::CornerRadius::same(cr.unwrap_or(8.0).max(8.0) as u8))
        .inner_margin(egui::Margin { left: 0, right: 0, top: 4, bottom: 4 })
        .show(ui, add_contents);

    ui.add_space(8.0);
}

pub fn slider_with_input(
    ui: &mut egui::Ui,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    step: f64,
    decimals: usize,
) -> egui::Response {
    let scale = get_scale(ui.ctx());

    // Number field width — 48dp is wide enough for "10.0" with compact margins.
    let number_w = 48.0 * scale;
    let gap      = ui.spacing().item_spacing.x;

    // Always use the real available width at paint time. grid_control_w is a
    // stale hint from a parent layout pass and causes sliders to be different
    // widths in the same scroll area.
    let total_w   = ui.available_width();
    // Subtract one thumb radius (10dp) so the thumb circle sits fully within
    // the slider allocation and doesn't visually touch the number field.
    let thumb_r   = 10.0 * scale;
    let slider_w  = (total_w - number_w - gap - thumb_r).max(40.0);

    let id_salt = (range.start().to_bits(), range.end().to_bits(), "slider_dv");

    let mut changed = false;
    let r = ui
        .horizontal(|ui| {
            // MaterialSlider allocates 32dp height internally. Pass 32dp here
            // so add_sized doesn't fight the widget's own allocation.
            let sr = ui.add_sized(
                [slider_w, 32.0],
                MaterialSlider::new(value, range.clone())
                    .width(slider_w)
                    .step(step as f32)
                    .show_value(false),
            );
            if sr.changed() {
                changed = true;
            }

            ui.add_space(thumb_r);

            let dv = ui
                .push_id(id_salt, |ui| {
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(number_w, 32.0),
                        egui::Sense::hover(),
                    );
                    ui.put(
                        rect,
                        MaterialNumberField::filled(value)
                            .range(range)
                            .decimals(decimals),
                    )
                })
                .inner;
            if dv.changed() {
                changed = true;
            }
            #[cfg(target_os = "android")]
            handle_android_keyboard(&dv, value);
        })
        .response;

    let mut r = r;
    if changed {
        r.mark_changed();
    }
    r
}


pub fn get_enum_options(class_name: &std::ffi::CStr) -> Vec<String> {
    use crate::il2cpp::{api::*, symbols::get_assembly_image, symbols::get_class};
    let mut options = Vec::new();
    let Ok(image) = get_assembly_image(c"umamusume.dll") else {
        return options;
    };
    let Ok(klass) = get_class(image, c"Gallop", class_name) else {
        return options;
    };

    if !il2cpp_class_is_enum(klass) {
        return options;
    }

    let mut iter: *mut std::ffi::c_void = std::ptr::null_mut();
    loop {
        let field = il2cpp_class_get_fields(klass, &mut iter);
        if field.is_null() {
            break;
        }
        let attrs = il2cpp_field_get_flags(field);
        if (attrs & 0x0040) != 0 {
            let name_ptr = il2cpp_field_get_name(field);
            if !name_ptr.is_null() {
                let name = unsafe { std::ffi::CStr::from_ptr(name_ptr) };
                if let Ok(s) = name.to_str() {
                    options.push(s.to_string());
                }
            }
        }
    }
    options
}


pub trait AppWindow {
    fn run(&mut self, ctx: &egui::Context) -> bool;
    fn plugin_window_id(&self) -> Option<i32> { None }
}
pub type BoxedWindow = Box<dyn AppWindow + Send>;

#[cfg(target_os = "android")]
pub fn handle_android_keyboard<T: 'static>(res: &egui::Response, val: &mut T) {
    {
        let Ok(mut owner_lock) = KEYBOARD_OWNER.try_lock() else {
            return;
        };
        if let Some(KeyboardOwner::JNI(_)) = *owner_lock {
            return;
        }

        if res.lost_focus() {
            if let Some(KeyboardOwner::Unity(id)) = *owner_lock {
                if id == res.id {
                    let kb_ptr = ACTIVE_KEYBOARD.load(Ordering::Acquire);
                    if !kb_ptr.is_null() {
                        TouchScreenKeyboard::set_active(kb_ptr, false);
                        ACTIVE_KEYBOARD.store(std::ptr::null_mut(), Ordering::Release);
                        *KEYBOARD_GC_HANDLE.lock().unwrap() = None;
                    }
                    *owner_lock = None;
                }
            }
            return;
        }
    }

    if !res.has_focus() {
        return;
    }

    use egui::{
        text::{CCursor, CCursorRange},
        widgets::text_edit::TextEditState,
    };
    use crate::core::utils::{char_to_utf16_index, utf16_to_char_index};

    let val_any = val as &dyn std::any::Any;
    PENDING_KB_TYPE.store(
        TouchScreenKeyboardType::KeyboardType::Default as i32,
        Ordering::Release,
    );

    let text = if let Some(s) = val_any.downcast_ref::<String>() {
        s.clone()
    } else if let Some(f) = val_any.downcast_ref::<f32>() {
        PENDING_KB_TYPE.store(
            TouchScreenKeyboardType::KeyboardType::DecimalPad as i32,
            Ordering::Release,
        );
        if f.fract() == 0.0 {
            format!("{:.1}", f)
        } else {
            f.to_string()
        }
    } else if let Some(i) = val_any.downcast_ref::<i32>() {
        PENDING_KB_TYPE.store(
            TouchScreenKeyboardType::KeyboardType::NumberPad as i32,
            Ordering::Release,
        );
        i.to_string()
    } else {
        String::new()
    };

    if res.gained_focus() {
        {
            let mut owner_lock = KEYBOARD_OWNER.lock().unwrap();
            *owner_lock = Some(KeyboardOwner::Unity(res.id));
        }

        res.scroll_to_me(Some(egui::Align::Center));

        let ptr = text.to_il2cpp_string();
        PENDING_KEYBOARD_TEXT.store(ptr, Ordering::Release);

        let initial_selection = res.ctx.data(|data| {
            data.get_temp::<TextEditState>(res.id)
                .and_then(|state| state.cursor.char_range())
                .map(|range| {
                    let start_char = range.primary.index.min(range.secondary.index);
                    let end_char = range.primary.index.max(range.secondary.index);

                    let start_u16 = char_to_utf16_index(&text, start_char);
                    let end_u16 = char_to_utf16_index(&text, end_char);

                    RangeInt::new(start_u16, end_u16 - start_u16)
                })
                .unwrap_or(RangeInt::new(
                    char_to_utf16_index(&text, text.chars().count()),
                    0,
                ))
        });
        *KEYBOARD_SELECTION.lock().unwrap() = initial_selection;

        Thread::main_thread().schedule(|| {
            let ptr = PENDING_KEYBOARD_TEXT.swap(std::ptr::null_mut(), Ordering::AcqRel);
            let typ: TouchScreenKeyboardType::KeyboardType = unsafe {
                *(&PENDING_KB_TYPE.load(Ordering::Acquire) as *const i32
                    as *const TouchScreenKeyboardType::KeyboardType)
            };

            if !ptr.is_null() {
                let keyboard = TouchScreenKeyboard::Open(ptr, typ, false, false, false);
                TouchScreenKeyboard::set_selection(keyboard, *KEYBOARD_SELECTION.lock().unwrap());
                let handle = GCHandle::new(keyboard, false);
                *KEYBOARD_GC_HANDLE.lock().unwrap() = Some(handle);
                ACTIVE_KEYBOARD.store(keyboard, Ordering::Release);
            }
        });
    }

    let kb_ptr = ACTIVE_KEYBOARD.load(Ordering::Acquire);
    if !kb_ptr.is_null() {
        let status = TouchScreenKeyboard::get_status(kb_ptr);

        if status == TouchScreenKeyboard::Status::Visible {
            let unity_range = TouchScreenKeyboard::get_selection(kb_ptr);

            let kb_txt_ptr = TouchScreenKeyboard::get_text(kb_ptr);
            if let Some(kb_ref) = unsafe { kb_txt_ptr.as_ref() } {
                let kb_txt_str = kb_ref.as_utf16str().to_string();

                let val_any_mut = val as &mut dyn std::any::Any;

                if let Some(s) = val_any_mut.downcast_mut::<String>() {
                    if *s != kb_txt_str {
                        *s = kb_txt_str.clone();
                    }
                } else if let Some(f) = val_any_mut.downcast_mut::<f32>() {
                    if let Ok(parsed) = kb_txt_str.parse::<f32>() {
                        let changed = !egui::emath::almost_equal(*f, parsed, 1e-6);
                        let drafting = kb_txt_str.ends_with('.')
                            || (kb_txt_str.contains('.') && kb_txt_str.ends_with('0'));

                        if changed && !drafting {
                            *f = parsed;
                        }
                    }
                } else if let Some(i) = val_any_mut.downcast_mut::<i32>() {
                    if let Ok(parsed) = kb_txt_str.parse::<i32>() {
                        if *i != parsed {
                            *i = parsed;
                        }
                    }
                }

                let kb_txt_clone = kb_txt_str.clone();
                res.ctx.data_mut(|data| {
                    if let Some(mut state) = data.get_temp::<TextEditState>(res.id) {
                        let start_char =
                            utf16_to_char_index(&kb_txt_clone, unity_range.start as usize);
                        let end_char = utf16_to_char_index(
                            &kb_txt_clone,
                            (unity_range.start + unity_range.length) as usize,
                        );

                        let new_range =
                            CCursorRange::two(CCursor::new(start_char), CCursor::new(end_char));

                        if state.cursor.char_range() != Some(new_range) {
                            state.cursor.set_char_range(Some(new_range));
                            data.insert_temp(res.id, state);
                        }
                    }
                });
            }
            res.ctx.request_repaint();
        }

        if status != TouchScreenKeyboard::Status::Visible {
            // On Done (OK tapped), do a final commit of whatever text is in the
            // keyboard before dismissing.  Canceled / LostFocus discard without
            // committing so the original value is preserved.
            if status == TouchScreenKeyboard::Status::Done {
                let kb_txt_ptr = TouchScreenKeyboard::get_text(kb_ptr);
                if let Some(kb_ref) = unsafe { kb_txt_ptr.as_ref() } {
                    let kb_txt_str = kb_ref.as_utf16str().to_string();
                    let val_any_mut = val as &mut dyn std::any::Any;

                    if let Some(s) = val_any_mut.downcast_mut::<String>() {
                        if *s != kb_txt_str {
                            *s = kb_txt_str;
                        }
                    } else if let Some(f) = val_any_mut.downcast_mut::<f32>() {
                        if let Ok(parsed) = kb_txt_str.parse::<f32>() {
                            *f = parsed;
                        }
                    } else if let Some(i) = val_any_mut.downcast_mut::<i32>() {
                        if let Ok(parsed) = kb_txt_str.parse::<i32>() {
                            *i = parsed;
                        }
                    }
                }
            }

            res.surrender_focus();
            res.ctx.memory_mut(|mem| mem.stop_text_input());
            res.ctx.data_mut(|data| {
                data.remove::<egui::widgets::text_edit::TextEditState>(res.id);
            });

            ACTIVE_KEYBOARD.store(std::ptr::null_mut(), Ordering::Release);
            *KEYBOARD_GC_HANDLE.lock().unwrap() = None;
            res.ctx.request_repaint();
        }
    }
}



pub struct Md3Snackbar {
    pub id: u32,
    pub message: String,
    pub persistent: bool,
    pub show_time: std::time::Instant,
}



