use crate::core::gui::*;

use egui_material3::*;
use egui_material3::theme::get_global_color;
use rust_i18n::t;

use crate::core::*;


pub struct ThemeEditorWindow {
    id: egui::Id,
    // Working copies of all theme fields
    seed: egui::Color32,
    old_seed: egui::Color32,
    theme_mode: hachimi::UiThemeMode,
    old_theme_mode: hachimi::UiThemeMode,
    contrast_level: hachimi::UiContrastLevel,
    old_contrast_level: hachimi::UiContrastLevel,
    scheme_mode: hachimi::UiColorSchemeMode,
    old_scheme_mode: hachimi::UiColorSchemeMode,
    surface_alpha: u8,
    old_surface_alpha: u8,
    window_rounding: f32,
    old_window_rounding: f32,
    rounding_mode: hachimi::UiRoundingMode,
    old_rounding_mode: hachimi::UiRoundingMode,
    translucency_mode: hachimi::UiTranslucencyMode,
    old_translucency_mode: hachimi::UiTranslucencyMode,
    // Manual color editing state
    manual_colors: std::collections::HashMap<String, [u8; 3]>,
    old_manual_colors: std::collections::HashMap<String, [u8; 3]>,
    /// Editable primary override for Manual mode
    manual_primary: egui::Color32,
    manual_secondary: egui::Color32,
    manual_tertiary: egui::Color32,
    manual_surface: egui::Color32,
    manual_on_surface: egui::Color32,
}


impl ThemeEditorWindow {
    pub fn new() -> ThemeEditorWindow {
        let cfg = (**Hachimi::instance().config.load()).clone();
        let manual_primary = cfg
            .ui_manual_colors
            .get("primary")
            .map(|&[r, g, b]| egui::Color32::from_rgb(r, g, b))
            .unwrap_or(egui_material3::theme::get_global_color("primary"));
        let manual_secondary = cfg
            .ui_manual_colors
            .get("secondary")
            .map(|&[r, g, b]| egui::Color32::from_rgb(r, g, b))
            .unwrap_or(egui_material3::theme::get_global_color("secondary"));
        let manual_tertiary = cfg
            .ui_manual_colors
            .get("tertiary")
            .map(|&[r, g, b]| egui::Color32::from_rgb(r, g, b))
            .unwrap_or(egui_material3::theme::get_global_color("tertiary"));
        let manual_surface = cfg
            .ui_manual_colors
            .get("surface")
            .map(|&[r, g, b]| egui::Color32::from_rgb(r, g, b))
            .unwrap_or(egui_material3::theme::get_global_color("surface"));
        let manual_on_surface = cfg
            .ui_manual_colors
            .get("onSurface")
            .map(|&[r, g, b]| egui::Color32::from_rgb(r, g, b))
            .unwrap_or(egui_material3::theme::get_global_color("onSurface"));
        ThemeEditorWindow {
            id: random_id(),
            seed: cfg.ui_theme_seed,
            old_seed: cfg.ui_theme_seed,
            theme_mode: cfg.ui_theme_mode,
            old_theme_mode: cfg.ui_theme_mode,
            contrast_level: cfg.ui_contrast_level,
            old_contrast_level: cfg.ui_contrast_level,
            scheme_mode: cfg.ui_color_scheme_mode,
            old_scheme_mode: cfg.ui_color_scheme_mode,
            surface_alpha: cfg.ui_surface_alpha,
            old_surface_alpha: cfg.ui_surface_alpha,
            window_rounding: cfg.ui_window_rounding,
            old_window_rounding: cfg.ui_window_rounding,
            rounding_mode: cfg.ui_rounding_mode,
            old_rounding_mode: cfg.ui_rounding_mode,
            translucency_mode: cfg.ui_translucency_mode,
            old_translucency_mode: cfg.ui_translucency_mode,
            manual_colors: cfg.ui_manual_colors.clone(),
            old_manual_colors: cfg.ui_manual_colors,
            manual_primary,
            manual_secondary,
            manual_tertiary,
            manual_surface,
            manual_on_surface,
        }
    }

    fn build_preview_config(&self) -> hachimi::Config {
        let mut cfg = (**Hachimi::instance().config.load()).clone();
        cfg.ui_theme_seed = self.seed;
        cfg.ui_theme_json = None; // force regeneration on preview
        cfg.ui_theme_mode = self.theme_mode;
        cfg.ui_contrast_level = self.contrast_level;
        cfg.ui_color_scheme_mode = self.scheme_mode;
        cfg.ui_manual_colors = self.manual_colors.clone();
        cfg.ui_surface_alpha = self.surface_alpha;
        cfg.ui_window_rounding = self.window_rounding;
        cfg.ui_rounding_mode = self.rounding_mode;
        cfg.ui_translucency_mode = self.translucency_mode;
        cfg
    }
}


impl AppWindow for ThemeEditorWindow {
    fn run(&mut self, ctx: &egui::Context) -> bool {
        let scale = get_scale(ctx);
        let mut open = true;
        let mut open2 = true;
        let mut save_clicked = false;
        let mut cancel_clicked = false;
        let mut reset_clicked = false;
        let mut preview_dirty = false;

        new_window(ctx, self.id, t!("theme_editor.title"))
            .open(&mut open)
            // Theme editor needs more width than the standard config editor
            // to fit the button row (Restore Defaults + Save + Cancel) without
            // truncation. Use 92% of viewport width, capped at 360 logical units.
            .fixed_size({
                let vp = ctx.viewport_rect();
                egui::vec2(
                    (vp.width() * 0.92).min(360.0 * scale),
                    (vp.height() * 0.68).min(300.0 * scale),
                )
            })
            .show(ctx, |ui| {
                ui.set_width(ui.max_rect().width());

                simple_window_layout(
                    ui,
                    self.id,
                    |ui| {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                            // Match ConfigEditor label color across all settings windows.
                            let on_surface_variant = get_global_color("onSurfaceVariant");
                            ui.style_mut().visuals.override_text_color = Some(on_surface_variant);

                            let section_frame = section_group_frame();

                            // ── Theme Mode ──────────────────────────────────────
                            ui.label(t!("theme_editor.theme_mode"));
                            ui.add_space(2.0);
                            let avail = ui.available_width();
                            ui.data_mut(|d| d.insert_temp(egui::Id::new("grid_control_w"), avail));
                            let dark_str = t!("theme_editor.dark");
                            let light_str = t!("theme_editor.light");
                            let theme_mode_choices = [
                                (hachimi::UiThemeMode::Dark, dark_str.as_ref()),
                                (hachimi::UiThemeMode::Light, light_str.as_ref()),
                            ];
                            if Gui::run_combo(ui, "theme_mode_combo", &mut self.theme_mode, &theme_mode_choices) {
                                preview_dirty = true;
                            }
                            ui.add_space(8.0);

                            // ── Contrast Level ───────────────────────────────────
                            ui.label(t!("theme_editor.contrast_level"));
                            ui.add_space(2.0);
                            let avail = ui.available_width();
                            ui.data_mut(|d| d.insert_temp(egui::Id::new("grid_control_w"), avail));
                            let normal_str = t!("theme_editor.normal");
                            let medium_str = t!("theme_editor.medium");
                            let high_str = t!("theme_editor.high");
                            let contrast_choices = [
                                (hachimi::UiContrastLevel::Normal, normal_str.as_ref()),
                                (hachimi::UiContrastLevel::Medium, medium_str.as_ref()),
                                (hachimi::UiContrastLevel::High, high_str.as_ref()),
                            ];
                            if Gui::run_combo(ui, "contrast_level_combo", &mut self.contrast_level, &contrast_choices) {
                                preview_dirty = true;
                            }
                            ui.add_space(8.0);

                            // ── Color Scheme Mode ────────────────────────────────
                            ui.label(t!("theme_editor.scheme_mode"));
                            ui.add_space(2.0);
                            let avail = ui.available_width();
                            ui.data_mut(|d| d.insert_temp(egui::Id::new("grid_control_w"), avail));
                            let auto_str = t!("theme_editor.auto");
                            let manual_str = t!("theme_editor.manual");
                            let scheme_choices = [
                                (hachimi::UiColorSchemeMode::Auto, auto_str.as_ref()),
                                (hachimi::UiColorSchemeMode::Manual, manual_str.as_ref()),
                            ];
                            if Gui::run_combo(ui, "scheme_mode_combo", &mut self.scheme_mode, &scheme_choices) {
                                preview_dirty = true;
                            }
                            ui.add_space(8.0);

                            // ── Seed Color ───────────────────────────────────────
                            ui.label(t!("theme_editor.seed_color"));
                            ui.add_space(2.0);
                            section_frame.show(ui, |ui| {
                                let res = custom_color_button_with_close(ui, &mut self.seed, "seed_color_popup_custom");
                                if res.changed() {
                                    if self.scheme_mode == hachimi::UiColorSchemeMode::Auto {
                                        preview_dirty = true;
                                    }
                                }
                            });
                            ui.add_space(8.0);

                            // ── Manual color overrides ───────────────────────────
                            if self.scheme_mode == hachimi::UiColorSchemeMode::Manual {
                                ui.label(t!("theme_editor.manual_colors"));
                                ui.add_space(2.0);
                                section_frame.show(ui, |ui| {
                                    // Primary
                                    ui.horizontal(|ui| {
                                        ui.label(t!("theme_editor.primary"));
                                        let res = custom_color_button_with_close(ui, &mut self.manual_primary, "manual_primary_popup_custom");
                                        if res.changed()
                                        {
                                            let c = self.manual_primary;
                                            self.manual_colors
                                                .insert("primary".into(), [c.r(), c.g(), c.b()]);
                                            preview_dirty = true;
                                        }
                                    });
                                    ui.add_space(4.0);
                                    // Secondary
                                    ui.horizontal(|ui| {
                                        ui.label(t!("theme_editor.secondary"));
                                        let res = custom_color_button_with_close(ui, &mut self.manual_secondary, "manual_secondary_popup_custom");
                                        if res.changed()
                                        {
                                            let c = self.manual_secondary;
                                            self.manual_colors
                                                .insert("secondary".into(), [c.r(), c.g(), c.b()]);
                                            preview_dirty = true;
                                        }
                                    });
                                    ui.add_space(4.0);
                                    // Tertiary
                                    ui.horizontal(|ui| {
                                        ui.label(t!("theme_editor.tertiary"));
                                        let res = custom_color_button_with_close(ui, &mut self.manual_tertiary, "manual_tertiary_popup_custom");
                                        if res.changed()
                                        {
                                            let c = self.manual_tertiary;
                                            self.manual_colors
                                                .insert("tertiary".into(), [c.r(), c.g(), c.b()]);
                                            preview_dirty = true;
                                        }
                                    });
                                    ui.add_space(4.0);
                                    // Surface
                                    ui.horizontal(|ui| {
                                        ui.label(t!("theme_editor.surface"));
                                        let res = custom_color_button_with_close(ui, &mut self.manual_surface, "manual_surface_popup_custom");
                                        if res.changed()
                                        {
                                            let c = self.manual_surface;
                                            self.manual_colors
                                                .insert("surface".into(), [c.r(), c.g(), c.b()]);
                                            preview_dirty = true;
                                        }
                                    });
                                    ui.add_space(4.0);
                                    // On Surface
                                    ui.horizontal(|ui| {
                                        ui.label(t!("theme_editor.on_surface"));
                                        let res = custom_color_button_with_close(ui, &mut self.manual_on_surface, "manual_on_surface_popup_custom");
                                        if res.changed()
                                        {
                                            let c = self.manual_on_surface;
                                            self.manual_colors
                                                .insert("onSurface".into(), [c.r(), c.g(), c.b()]);
                                            preview_dirty = true;
                                        }
                                    });
                                });
                                ui.add_space(8.0);
                            }

                            // ── Background Transparency ───────────────────────────
                            ui.label(t!("theme_editor.transparency"));
                            let mut alpha_f = self.surface_alpha as f32 / 255.0;
                            if slider_with_input(ui, &mut alpha_f, 0.0..=1.0, 0.1, 2).changed()
                            {
                                self.surface_alpha = (alpha_f * 255.0).round() as u8;
                                preview_dirty = true;
                            }
                            ui.add_space(8.0);

                            // ── Corner Radius ─────────────────────────────────────
                            ui.label(t!("theme_editor.corner_radius"));
                            if slider_with_input(ui, &mut self.window_rounding, 0.0..=20.0, 0.5, 1)
                                .changed()
                            {
                                preview_dirty = true;
                            }
                            ui.add_space(4.0);
                            ui.label(t!("theme_editor.rounding_mode"));
                            ui.add_space(2.0);
                            let avail = ui.available_width();
                            ui.data_mut(|d| d.insert_temp(egui::Id::new("grid_control_w"), avail));
                            let round_all = t!("theme_editor.rounding_all");
                            let round_cards = t!("theme_editor.rounding_cards");
                            let round_windows = t!("theme_editor.rounding_windows");
                            let rounding_mode_choices = [
                                (hachimi::UiRoundingMode::All, round_all.as_ref()),
                                (hachimi::UiRoundingMode::Cards, round_cards.as_ref()),
                                (hachimi::UiRoundingMode::Windows, round_windows.as_ref()),
                            ];
                            if Gui::run_combo(ui, "rounding_mode_combo", &mut self.rounding_mode, &rounding_mode_choices) {
                                preview_dirty = true;
                            }
                            ui.add_space(8.0);

                            // ── Translucency Method ───────────────────────────────
                            ui.label(t!("theme_editor.translucency_method"));
                            ui.add_space(2.0);
                            let avail = ui.available_width();
                            ui.data_mut(|d| d.insert_temp(egui::Id::new("grid_control_w"), avail));
                            let trans_none = t!("theme_editor.translucency_none");
                            let trans_overlay = t!("theme_editor.translucency_overlay");
                            let trans_full = t!("theme_editor.translucency_full");
                            let translucency_choices = [
                                (hachimi::UiTranslucencyMode::None, trans_none.as_ref()),
                                (hachimi::UiTranslucencyMode::Overlay, trans_overlay.as_ref()),
                                (hachimi::UiTranslucencyMode::Full, trans_full.as_ref()),
                            ];
                            if Gui::run_combo(ui, "translucency_mode_combo", &mut self.translucency_mode, &translucency_choices) {
                                preview_dirty = true;
                            }
                            ui.add_space(8.0);
                        });
                    },
                    |ui| {
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                            let error_col = get_global_color("error");
                            if ui
                                .add(MaterialButton::text(t!("config_editor.restore_defaults"))
                                    .truncate()
                                    .text_color(error_col))
                                .clicked()
                            {
                                reset_clicked = true;
                            }
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                                if ui.add(MaterialButton::outlined(t!("cancel"))).clicked() {
                                    cancel_clicked = true;
                                    open2 = false;
                                }
                                if ui.add(MaterialButton::filled(t!("save"))).clicked() {
                                    save_clicked = true;
                                    open2 = false;
                                }
                            });
                        });
                    },
                );
            });

        // Live preview whenever any field changed this frame
        if preview_dirty {
            enqueue_theme_preview(self.build_preview_config());
        }

        if save_clicked {
            let mut cfg = self.build_preview_config();
            let data_dir = Hachimi::instance().game.data_dir.clone();
            let manual = cfg.ui_manual_colors.clone();
            let save_params = crate::core::theme::ThemeParams {
                seed: cfg.ui_theme_seed,
                cached_json: None,
                theme_mode: cfg.ui_theme_mode,
                contrast_level: cfg.ui_contrast_level,
                scheme_mode: cfg.ui_color_scheme_mode,
                manual_colors: &manual,
                surface_alpha: cfg.ui_surface_alpha,
                window_rounding: cfg.ui_window_rounding,
                rounding_mode: cfg.ui_rounding_mode,
            };
            if let Some(json) = crate::core::theme::apply_seed(ctx, save_params, &data_dir) {
                cfg.ui_theme_json = serde_json::from_str(&json).ok();
            }
            save_and_reload_config(cfg);
        }
        if cancel_clicked {
            // Restore all fields to old values and re-preview
            self.seed = self.old_seed;
            self.theme_mode = self.old_theme_mode;
            self.contrast_level = self.old_contrast_level;
            self.scheme_mode = self.old_scheme_mode;
            self.surface_alpha = self.old_surface_alpha;
            self.window_rounding = self.old_window_rounding;
            self.rounding_mode = self.old_rounding_mode;
            self.translucency_mode = self.old_translucency_mode;
            self.manual_colors = self.old_manual_colors.clone();
            enqueue_theme_preview(self.build_preview_config());
        }
        if reset_clicked {
            self.seed = hachimi::Config::default_ui_theme_seed();
            self.theme_mode = hachimi::UiThemeMode::Dark;
            self.contrast_level = hachimi::UiContrastLevel::Normal;
            self.scheme_mode = hachimi::UiColorSchemeMode::Auto;
            self.surface_alpha = hachimi::Config::default_ui_surface_alpha();
            self.window_rounding = hachimi::Config::default_ui_window_rounding();
            self.rounding_mode = hachimi::UiRoundingMode::All;
            self.translucency_mode = hachimi::UiTranslucencyMode::None;
            self.manual_colors.clear();
            self.manual_primary = hachimi::Config::default_ui_theme_seed();
            self.manual_secondary = egui_material3::theme::get_global_color("secondary");
            self.manual_tertiary = egui_material3::theme::get_global_color("tertiary");
            self.manual_surface = egui::Color32::from_rgb(22, 18, 13);
            self.manual_on_surface = egui::Color32::from_rgb(238, 225, 212);
            enqueue_theme_preview(self.build_preview_config());
        }

        open & open2
    }

    fn window_id(&self) -> Option<egui::Id> {
        Some(self.id)
    }
}
