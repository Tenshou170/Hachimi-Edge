use crate::core::gui::config::ConfigEditor;
#[allow(unused_imports)]
use crate::core::gui::utils::LIST_TILE_H;
use crate::core::gui::Gui;
use crate::core::gui::dialogs::SimpleOkDialog;
use crate::core::gui::windows::theme::ThemeEditorWindow;
use crate::core::hachimi::*;
#[allow(unused_imports)]
use egui_material3::{MaterialButton, MaterialSwitch};
#[cfg(target_os = "windows")]
use egui_material3::theme::get_global_color;
use rust_i18n::t;
use std::thread;


pub fn render(_editor: &ConfigEditor, config: &mut crate::core::hachimi::Config, ui: &mut egui::Ui) {

    let lang_changed = ConfigEditor::list_tile_combo(
        ui, t!("config_editor.language"), "language", &mut config.language, Language::CHOICES,
    );
    if lang_changed { config.language.set_locale(); }

    if ConfigEditor::list_tile_switch(ui, t!("config_editor.disable_overlay"), &mut config.disable_gui, true)
        .clicked() && config.disable_gui
    {
        thread::spawn(|| {
            Gui::instance().unwrap().lock().unwrap_or_else(|e| e.into_inner())
                .show_window(Box::new(SimpleOkDialog::new(
                    &t!("warning"), &t!("config_editor.disable_overlay_warning"), || {},
                )));
        });
    }

    ConfigEditor::list_tile_slider(ui, t!("config_editor.gui_scale"), &mut config.gui_scale, 0.25..=2.0, 0.05, 2);

    #[cfg(target_os = "android")]
    {
        ConfigEditor::list_tile_switch(ui, t!("config_editor.gui_landscape_ratio"),
            &mut config.android.enable_gui_landscape_ratio, true);
        if config.android.enable_gui_landscape_ratio {
            ConfigEditor::list_tile_slider(ui, "", &mut config.android.gui_landscape_ratio, 0.25..=2.0, 0.05, 2);
        }
    }

    #[cfg(target_os = "windows")]
    {
        ConfigEditor::list_tile_switch(ui, t!("config_editor.gui_landscape_ratio"),
            &mut config.windows.enable_gui_landscape_ratio, true);
        if config.windows.enable_gui_landscape_ratio {
            ConfigEditor::list_tile_slider(ui, "", &mut config.windows.gui_landscape_ratio, 0.25..=2.0, 0.05, 2);
        }

        // Hotkey: setting name on top, chip + "Set" button on the row below
        {
            let key_label = crate::windows::utils::vk_to_display_label(config.windows.menu_open_key);
            let secondary_container    = get_global_color("secondaryContainer");
            let on_secondary_container = get_global_color("onSecondaryContainer");

            // Measure chip width from the key text
            let key_galley = ui.painter().layout_no_wrap(
                key_label.to_string(),
                ui.style().text_styles[&egui::TextStyle::Body].clone(),
                egui::Color32::WHITE,
            );
            let chip_w = key_galley.size().x + 16.0;

            // Top line: setting name label
            ui.add(egui::Label::new(t!("config_editor.menu_open_key")).wrap());

            // Bottom line: chip left, "Set" button right
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                // Key chip — secondaryContainer pill
                let (chip_rect, _) = ui.allocate_exact_size(
                    egui::vec2(chip_w, 28.0),
                    egui::Sense::hover(),
                );
                ui.painter().rect_filled(chip_rect, 6.0, secondary_container);
                ui.painter().text(
                    chip_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    key_label,
                    ui.style().text_styles[&egui::TextStyle::Body].clone(),
                    on_secondary_container,
                );
                // "Set" button right-aligned
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add(MaterialButton::outlined(t!("config_editor.menu_open_key_set"))).clicked() {
                        crate::windows::wnd_hook::start_menu_key_capture();
                        thread::spawn(|| {
                            Gui::instance().unwrap().lock().unwrap_or_else(|e| e.into_inner())
                                .show_notification(&t!("notification.press_to_set_menu_key"));
                        });
                    }
                });
            });
            ui.add_space(4.0);
        }
    }

    // Theme Settings
    if ConfigEditor::list_tile_action_button(ui, t!("theme_editor.title"), t!("open")) {
        thread::spawn(|| {
            Gui::instance().unwrap().lock().unwrap_or_else(|e| e.into_inner())
                .show_window(Box::new(ThemeEditorWindow::new()));
        });
    }
}
