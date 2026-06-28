use crate::core::gui::*;

use egui_material3::*;
use egui_material3::theme::get_global_color;
use rust_i18n::t;

use crate::core::*;
use std::{sync::Arc, ops::RangeInclusive};


#[derive(Eq, PartialEq, Clone, Copy)]
pub enum ConfigEditorTab {
    General,
    Graphics,
    Gameplay,
    Advanced,
}

impl ConfigEditorTab {
    pub fn display_list() -> [(ConfigEditorTab, Cow<'static, str>); 4] {
        [
            (ConfigEditorTab::General,  t!("config_editor.general_tab")),
            (ConfigEditorTab::Graphics, t!("config_editor.graphics_tab")),
            (ConfigEditorTab::Gameplay, t!("config_editor.gameplay_tab")),
            (ConfigEditorTab::Advanced, t!("config_editor.advanced_tab")),
        ]
    }

    pub fn as_index(self) -> usize {
        match self {
            ConfigEditorTab::General  => 0,
            ConfigEditorTab::Graphics => 1,
            ConfigEditorTab::Gameplay => 2,
            ConfigEditorTab::Advanced => 3,
        }
    }

    pub fn from_index(i: usize) -> Self {
        match i {
            1 => ConfigEditorTab::Graphics,
            2 => ConfigEditorTab::Gameplay,
            3 => ConfigEditorTab::Advanced,
            _ => ConfigEditorTab::General,
        }
    }
}

pub struct ConfigEditor {
    pub last_ptr_config:    usize,
    pub config:             hachimi::Config,
    pub id:                 egui::Id,
    pub current_tab:        ConfigEditorTab,
    pub champions_resources: Vec<String>,
    pub font_color_options:  Vec<String>,
    pub outline_size_options: Vec<String>,
    pub outline_color_options: Vec<String>,
    pub localized_data_dirs: Vec<String>,
}

impl ConfigEditor {
    pub fn new() -> ConfigEditor {
        let handle = Hachimi::instance().config.load();
        let config = (**handle).clone();
        let localized_data_dirs =
            Self::load_localized_data_dirs(config.localized_data_dir.as_deref());
        ConfigEditor {
            last_ptr_config: Arc::as_ptr(&handle) as usize,
            config,
            id: random_id(),
            current_tab: ConfigEditorTab::General,
            champions_resources:   crate::il2cpp::sql::get_champions_resources(),
            font_color_options:    get_enum_options(c"FontColorType"),
            outline_size_options:  get_enum_options(c"OutlineSizeType"),
            outline_color_options: get_enum_options(c"OutlineColorType"),
            localized_data_dirs,
        }
    }

    fn load_localized_data_dirs(current: Option<&str>) -> Vec<String> {
        let hachimi = Hachimi::instance();
        let data_path = hachimi.get_data_path("");
        let mut available_dirs: Vec<String> = std::fs::read_dir(&data_path)
            .map(|rd| {
                rd.filter_map(|e| {
                    let e = e.ok()?;
                    if e.file_type().ok()?.is_dir() {
                        e.file_name().into_string().ok()
                    } else {
                        None
                    }
                })
                .collect()
            })
            .unwrap_or_default();

        available_dirs.sort_unstable();
        available_dirs.dedup();

        let default_dir = "localized_data".to_string();
        if !available_dirs.iter().any(|dir| dir == &default_dir) {
            available_dirs.insert(0, default_dir);
        }
        if let Some(current) = current {
            if !current.is_empty() && !available_dirs.iter().any(|dir| dir == current) {
                available_dirs.push(current.to_owned());
            }
        }

        available_dirs
    }

    pub fn restore_defaults(&mut self) {
        let current_language = self.config.language;
        self.config = hachimi::Config::default();
        self.config.language = current_language;
    }

    // ── Single renderer — used by both portrait and landscape ──────────────────

    pub fn run_options(
        &self,
        config: &mut hachimi::Config,
        ui: &mut egui::Ui,
        tab: ConfigEditorTab,
    ) {
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
        match tab {
            ConfigEditorTab::General  => super::tabs::general::render(self, config, ui),
            ConfigEditorTab::Graphics => super::tabs::graphics::render(self, config, ui),
            ConfigEditorTab::Gameplay => super::tabs::gameplay::render(self, config, ui),
            ConfigEditorTab::Advanced => super::tabs::advanced::render(self, config, ui),
        }
    }

    // ── list_tile_* helpers ────────────────────────────────────────────────────
    //
    // Each helper renders a full-width MD3 "list tile" row:
    //   • 16dp horizontal padding on both sides
    //   • minimum 48dp row height (MD3 touch target)
    //   • label on the left (wrapping), control right-aligned
    //   • 4dp vertical spacing after each row (callers can override)
    //
    // These replace the old stacked_* and option_slider grid helpers.

    /// Boolean toggle row: label left-aligned (wrapping), `MaterialSwitch` right.
    /// Returns the Switch `Response` so callers can check `.clicked()`.
    pub fn list_tile_switch(
        ui: &mut egui::Ui,
        label: impl Into<egui::WidgetText>,
        value: &mut bool,
        enabled: bool,
    ) -> egui::Response {
        ui.vertical(|ui| {
            let row_resp = ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                let avail    = ui.available_width();
                let switch_w = 52.0;
                let label_w  = (avail - switch_w - 8.0).max(40.0);

                ui.allocate_ui_with_layout(
                    egui::vec2(label_w, LIST_TILE_H),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| { ui.add(egui::Label::new(label).wrap()); },
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add(MaterialSwitch::new(value).enabled(enabled))
                }).inner
            }).inner;

            ui.add_space(2.0);
            row_resp
        }).inner
    }

    /// Slider row: label on top, full-width `MaterialSlider` + number field below.
    pub fn list_tile_slider(
        ui: &mut egui::Ui,
        label: impl Into<egui::WidgetText>,
        value: &mut f32,
        range: std::ops::RangeInclusive<f32>,
        step: f64,
        decimals: usize,
    ) -> egui::Response {
        ui.vertical(|ui| {
            ui.add(egui::Label::new(label).wrap());
            let r = slider_with_input(ui, value, range, step, decimals);
            ui.add_space(4.0);
            r
        }).inner
    }

    /// Dropdown row: label on top, full-width `MaterialSelect` below.
    /// Width is set to `available_width()` so it always fills the content column.
    pub fn list_tile_combo<T: PartialEq + Copy>(
        ui: &mut egui::Ui,
        label: impl Into<egui::WidgetText>,
        id_child: impl std::hash::Hash,
        value: &mut T,
        choices: &[(T, &str)],
    ) -> bool {
        ui.vertical(|ui| {
            ui.add(egui::Label::new(label).wrap());
            let avail = ui.available_width();
            ui.data_mut(|d| d.insert_temp(egui::Id::new("grid_control_w"), avail));
            let changed = Gui::run_combo(ui, id_child, value, choices);
            ui.add_space(4.0);
            changed
        }).inner
    }

    /// Text-field row: label on top, full-width `MaterialTextField` below.
    pub fn list_tile_text_field(
        ui: &mut egui::Ui,
        label: impl Into<egui::WidgetText>,
        value: &mut String,
    ) -> egui::Response {
        ui.vertical(|ui| {
            ui.add(egui::Label::new(label).wrap());
            ui.set_max_width(ui.available_width());
            let r = ui.add(MaterialTextField::filled(value));
            ui.add_space(4.0);
            r
        }).inner
    }

    /// Option-slider row: switch inline (enable/disable), then slider below when on.
    /// Returns the Switch response so callers can detect the toggle event.
    pub fn list_tile_option_slider<Num: egui::emath::Numeric>(
        ui: &mut egui::Ui,
        label: &str,
        value: &mut Option<Num>,
        range: RangeInclusive<Num>,
    ) -> egui::Response {
        ui.vertical(|ui| {
            let mut checked = value.is_some();
            let sw_resp = Self::list_tile_switch(ui, label, &mut checked, true);

            if checked && value.is_none() {
                *value = Some(*range.start());
            } else if !checked && value.is_some() {
                *value = None;
            }

            if let Some(num) = value.as_mut() {
                let mut val_f  = num.to_f64() as f32;
                let range_f    = (range.start().to_f64() as f32)..=(range.end().to_f64() as f32);
                if slider_with_input(ui, &mut val_f, range_f, 1.0, 0).changed() {
                    *num = Num::from_f64(val_f as f64);
                }
                ui.add_space(4.0);
            }

            sw_resp
        }).inner
    }

    /// Button row: label left-aligned (wrapping), filled tonal button right.
    /// Returns `true` if the button was clicked.
    pub fn list_tile_button(
        ui: &mut egui::Ui,
        label: impl Into<egui::WidgetText>,
        btn_label: impl Into<egui::WidgetText>,
    ) -> bool {
        ui.vertical(|ui| {
            let clicked = ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                let avail   = ui.available_width();
                let btn_w   = 80.0_f32;
                let label_w = (avail - btn_w - 8.0).max(40.0);

                ui.allocate_ui_with_layout(
                    egui::vec2(label_w, LIST_TILE_H),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| { ui.add(egui::Label::new(label).wrap()); },
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add(MaterialButton::filled_tonal(btn_label)).clicked()
                }).inner
            }).inner;

            ui.add_space(2.0);
            clicked
        }).inner
    }

    /// Danger button row: label left-aligned (wrapping), red filled error button right.
    /// Returns `true` if the button was clicked.
    pub fn list_tile_button_danger(
        ui: &mut egui::Ui,
        label: impl Into<egui::WidgetText>,
        btn_label: impl Into<egui::WidgetText>,
    ) -> bool {
        ui.vertical(|ui| {
            let clicked = ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                let avail   = ui.available_width();
                let btn_w   = 80.0_f32;
                let label_w = (avail - btn_w - 8.0).max(40.0);

                ui.allocate_ui_with_layout(
                    egui::vec2(label_w, LIST_TILE_H),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| { ui.add(egui::Label::new(label).wrap()); },
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add(
                        MaterialButton::filled(btn_label)
                            .fill(get_global_color("error"))
                            .text_color(get_global_color("onError"))
                    ).clicked()
                }).inner
            }).inner;

            ui.add_space(2.0);
            clicked
        }).inner
    }

    pub fn list_tile_info(
        ui: &mut egui::Ui,
        label: impl Into<egui::WidgetText>,
        value: impl Into<egui::WidgetText>,
    ) {
        ui.vertical(|ui| {
            let value: egui::WidgetText = value.into();
            // Measure value text so label column gets all remaining space.
            let val_str = value.text().to_string();
            let val_galley = ui.painter().layout_no_wrap(
                val_str,
                ui.style().text_styles[&egui::TextStyle::Body].clone(),
                egui::Color32::WHITE,
            );
            let val_w   = (val_galley.size().x + 16.0).max(40.0);
            let avail   = ui.available_width();
            let label_w = (avail - val_w - 8.0).max(40.0);

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                ui.allocate_ui_with_layout(
                    egui::vec2(label_w, LIST_TILE_H),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| { ui.add(egui::Label::new(label).wrap()); },
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add(egui::Label::new(value).wrap());
                });
            });

            ui.add_space(2.0);
        });
    }

    /// String-select row: label on top, full-width outlined `MaterialSelect` below.
    /// Identical style to `list_tile_combo` but works with `&[String]` options
    /// instead of the generic `&[(T, &str)]`.
    pub fn list_tile_string_select(
        ui: &mut egui::Ui,
        label: impl Into<egui::WidgetText>,
        id_salt: impl std::hash::Hash,
        current: &mut String,
        options: &[String],
    ) {
        ui.add(egui::Label::new(label).wrap());
        let avail = ui.available_width();
        ui.data_mut(|d| d.insert_temp(egui::Id::new("grid_control_w"), avail));
        let mut sel = options.iter().position(|o| o == current);
        let mut select = MaterialSelect::new(&mut sel)
            .variant(SelectVariant::Outlined)
            .placeholder(current.as_str())
            .width(avail)
            .small();
        for (i, o) in options.iter().enumerate() { select = select.option(i, o); }
        if ui.push_id(id_salt, |ui| ui.add(select)).inner.changed() {
            if let Some(i) = sel { *current = options[i].clone(); }
        }
        ui.add_space(4.0);
    }

    /// Number-field row: label on top, full-width `MaterialNumberField` below.
    /// `suffix` is optional (e.g. `"ms"`, `"w"`, `"h"`).
    pub fn list_tile_number<Num: egui::emath::Numeric>(
        ui: &mut egui::Ui,
        label: impl Into<egui::WidgetText>,
        value: &mut Num,
        range: std::ops::RangeInclusive<Num>,
        speed: f64,
        suffix: Option<&str>,
    ) {
        ui.add(egui::Label::new(label).wrap());
        let mut field = MaterialNumberField::filled(value)
            .range(range)
            .speed(speed);
        if Num::INTEGRAL {
            field = field.decimals(0);
        }
        if let Some(s) = suffix { field = field.suffix(s); }
        ui.add(field);
        ui.add_space(4.0);
    }

    /// Supporting text row — rendered below a setting like the body text in
    /// an Android settings app. Indented 16dp, 11sp, `onSurfaceVariant` color.
    /// Visually subordinate to the preceding setting name.
    pub fn list_tile_hint(ui: &mut egui::Ui, text: impl Into<egui::WidgetText>) {
        let color = egui_material3::theme::get_global_color("onSurfaceVariant");
        let text_str = text.into().text().to_string();
        // Clear override_text_color so the explicit RichText color always wins
        // even when the parent tab sets override_text_color = onSurfaceVariant.
        ui.scope(|ui| {
            ui.visuals_mut().override_text_color = None;
            ui.add(egui::Label::new(
                egui::RichText::new(text_str)
                    .size(11.5)
                    .color(color)
            ).wrap());
        });
        ui.add_space(6.0);
    }

    // ── Legacy grid helpers (kept for backward compat during migration) ─────────

    /// Grid-mode option renderer used by the landscape layout (legacy path).
    /// Prefer `list_tile_option_slider` in new code.
    #[allow(dead_code)]
    pub fn option_slider<Num: egui::emath::Numeric>(
        ui: &mut egui::Ui,
        label: &str,
        value: &mut Option<Num>,
        range: RangeInclusive<Num>,
    ) {
        Self::list_tile_option_slider(ui, label, value, range);
    }

    // ── Backward-compat aliases (old stacked_* names → new list_tile_* bodies) ──

    /// Alias for `list_tile_switch`.
    pub fn stacked_checkbox(
        ui: &mut egui::Ui,
        label: impl Into<egui::WidgetText>,
        value: &mut bool,
        enabled: bool,
    ) -> egui::Response {
        Self::list_tile_switch(ui, label, value, enabled)
    }

    /// Alias for `list_tile_slider`.
    pub fn stacked_slider(
        ui: &mut egui::Ui,
        label: impl Into<egui::WidgetText>,
        value: &mut f32,
        range: std::ops::RangeInclusive<f32>,
        step: f64,
        decimals: usize,
    ) -> egui::Response {
        Self::list_tile_slider(ui, label, value, range, step, decimals)
    }

    /// Alias for `list_tile_combo`.
    pub fn stacked_combo<T: PartialEq + Copy>(
        ui: &mut egui::Ui,
        label: impl Into<egui::WidgetText>,
        id_child: impl std::hash::Hash,
        value: &mut T,
        choices: &[(T, &str)],
    ) -> bool {
        Self::list_tile_combo(ui, label, id_child, value, choices)
    }

    /// Alias for `list_tile_text_field`.
    pub fn stacked_text_field(
        ui: &mut egui::Ui,
        label: impl Into<egui::WidgetText>,
        value: &mut String,
    ) -> egui::Response {
        Self::list_tile_text_field(ui, label, value)
    }

    /// Alias for `list_tile_option_slider`.
    pub fn stacked_option_slider<Num: egui::emath::Numeric>(
        ui: &mut egui::Ui,
        label: &str,
        value: &mut Option<Num>,
        range: RangeInclusive<Num>,
    ) {
        Self::list_tile_option_slider(ui, label, value, range);
    }
}
