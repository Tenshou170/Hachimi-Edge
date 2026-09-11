use crate::core::gui::config::ConfigEditor;
use crate::core::gui::Gui;
use crate::core::gui::dialogs::SimpleOkDialog;
use crate::core::gui::windows::live_vocals::LiveVocalsSwapWindow;
use crate::core::Hachimi;
#[allow(unused_imports)]
use egui_material3::*;
use rust_i18n::t;
use std::thread;
use crate::il2cpp::hook::umamusume::CySpringController::SpringUpdateMode;
use crate::il2cpp::hook::umamusume::TimeUtil::BgSeason;

#[allow(unused_variables)]
pub fn render(editor: &ConfigEditor, config: &mut crate::core::hachimi::Config, ui: &mut egui::Ui) {

    ConfigEditor::list_tile_combo(ui, t!("config_editor.physics_update_mode"), "physics_update_mode",
        &mut config.physics_update_mode, &[
            (None, &t!("default")),
            (SpringUpdateMode::ModeNormal.into(), "ModeNormal"),
            (SpringUpdateMode::Mode60FPS.into(), "Mode60FPS"),
            (SpringUpdateMode::SkipFrame.into(), "SkipFrame"),
            (SpringUpdateMode::SkipFramePostAlways.into(), "SkipFramePostAlways"),
        ]);
    ConfigEditor::list_tile_switch(ui, t!("config_editor.cyspring_mono_uncap_frame_scale"), &mut config.cyspring_mono_uncap_frame_scale, true);
    if config.cyspring_mono_uncap_frame_scale {
        ConfigEditor::list_tile_switch_described(ui, t!("config_editor.cyspring_disable_native"), &mut config.cyspring_disable_native, true, t!("config_editor.cyspring_disable_native_desc"));
        ConfigEditor::list_tile_slider(ui, t!("config_editor.cyspring_stiffness_force_rate_scale"), &mut config.cyspring_stiffness_force_rate_scale, 0.1..=3.0, 0.05, 2);
        ConfigEditor::list_tile_slider(ui, t!("config_editor.cyspring_drag_force_rate_scale"), &mut config.cyspring_drag_force_rate_scale, 0.1..=3.0, 0.05, 2);
    }
    ConfigEditor::list_tile_slider(ui, t!("config_editor.story_choice_auto_select_delay"), &mut config.story_choice_auto_select_delay, 0.1..=10.0, 0.05, 2);
    ConfigEditor::list_tile_slider(ui, t!("config_editor.story_text_speed_multiplier"), &mut config.story_tcps_multiplier, 0.1..=10.0, 0.1, 1);
    ConfigEditor::list_tile_switch(ui, t!("config_editor.force_allow_dynamic_camera"), &mut config.force_allow_dynamic_camera, true);
    ConfigEditor::list_tile_switch(ui, t!("config_editor.live_theater_allow_same_chara"), &mut config.live_theater_allow_same_chara, true);
    ConfigEditor::list_tile_switch(ui, t!("config_editor.disable_tap_effect"), &mut config.disable_tap_effect, true);

    if ConfigEditor::list_tile_action_button(ui, t!("config_editor.live_vocals_swap"), t!("open")) {
        thread::spawn(|| {
            Gui::instance().unwrap().lock().unwrap_or_else(|e| e.into_inner())
                .show_window(Box::new(LiveVocalsSwapWindow::new()));
        });
    }

    #[cfg(target_os = "windows")]
    {
        if ConfigEditor::list_tile_action_button(ui, t!("config_editor.free_camera"), t!("open")) {
            thread::spawn(|| {
                let Some(gui_mutex) = Gui::instance() else { return };
                let mut gui = gui_mutex.lock().unwrap_or_else(|e| e.into_inner());
                gui.show_window(Box::new(crate::core::gui::windows::FreeCameraWindow::new()));
            });
        }
        if ConfigEditor::list_tile_action_button(ui, t!("free_camera.cheatsheet_title"), t!("open")) {
            thread::spawn(|| {
                let Some(gui_mutex) = Gui::instance() else { return };
                let mut gui = gui_mutex.lock().unwrap_or_else(|e| e.into_inner());
                gui.show_window(Box::new(crate::core::gui::dialogs::SimpleMarkdownDialog::new_with_height(
                    &t!("free_camera.cheatsheet_title"),
                    &t!("free_camera.cheatsheet_contents"),
                    400.0,
                    500.0,
                )));
            });
        }
    }

    if Hachimi::instance().game.region == crate::core::game::Region::Japan {
        ConfigEditor::list_tile_switch(ui, t!("config_editor.skill_info_dialog"), &mut config.skill_info_dialog, true);
    }

    {
        let localized_data = Hachimi::instance().localized_data.load();
        let get_season = |k, default: &str| localized_data.localize_dict.get(k).map(|s| s.as_str()).unwrap_or(default).to_string();

        let season_spring = get_season("Common0108", &t!("spring"));
        let season_summer = get_season("Common0109", &t!("summer"));
        let season_fall = get_season("Common0110", &t!("fall"));
        let season_winter = get_season("Common0111", &t!("winter"));
        let season_sakura = get_season("Common0112", &t!("cherry_blossom"));

        ConfigEditor::list_tile_combo(ui, t!("config_editor.homescreen_bgseason"), "homescreen_bgseason",
            &mut config.homescreen_bgseason, &[
                (BgSeason::None, &t!("default")),
                (BgSeason::Spring, season_spring.as_str()),
                (BgSeason::Summer, season_summer.as_str()),
                (BgSeason::Fall, season_fall.as_str()),
                (BgSeason::Winter, season_winter.as_str()),
                (BgSeason::CherryBlossom, season_sakura.as_str()),
            ]);
    }
    ConfigEditor::list_tile_switch(ui, t!("config_editor.disable_skill_name_translation"), &mut config.disable_skill_name_translation, true);

    if ConfigEditor::list_tile_switch(ui, t!("config_editor.hide_ingame_ui_hotkey"), &mut config.hide_ingame_ui_hotkey, true)
        .clicked() && config.hide_ingame_ui_hotkey
    {
        thread::spawn(|| {
            Gui::instance().unwrap().lock().unwrap_or_else(|e| e.into_inner())
                .show_window(Box::new(SimpleOkDialog::new(
                    &t!("info"), &t!("config_editor.hide_ingame_ui_hotkey_info"), || {},
                )));
        });
    }

    if config.hide_ingame_ui_hotkey && !ConfigEditor::row_filtered(&t!("config_editor.hide_ingame_ui_hotkey_bind")) {
        ConfigEditor::maybe_draw_category_header(ui);
        #[cfg(target_os = "windows")]
        let key_label = crate::windows::utils::vk_to_display_label(config.windows.hide_ingame_ui_hotkey_bind);
        #[cfg(target_os = "android")]
        let key_label = crate::android::gui_impl::keymap::keycode_display_label(config.android.hide_ingame_ui_hotkey_bind);
        let secondary_container    = egui_material3::theme::get_global_color("secondaryContainer");
        let on_secondary_container = egui_material3::theme::get_global_color("onSecondaryContainer");
        let key_galley = ui.painter().layout_no_wrap(
            key_label.to_string(),
            ui.style().text_styles[&egui::TextStyle::Body].clone(),
            egui::Color32::WHITE,
        );
        let chip_w = key_galley.size().x + 16.0;

        ui.add(egui::Label::new(t!("config_editor.hide_ingame_ui_hotkey_bind")).wrap());
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
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
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add(MaterialButton::outlined(t!("bind_key"))).clicked() {
                    thread::spawn(|| {
                        let Some(gui_mutex) = Gui::instance() else { return };
                        let mut gui = gui_mutex.lock().unwrap_or_else(|e| e.into_inner());
                        gui.show_window(Box::new(crate::core::gui::windows::SetKeybindWindow::new(|result| {
                            let Some(raw) = result else { return };
                            let hachimi = crate::core::Hachimi::instance();
                            let mut new_config = hachimi.config.load().as_ref().clone();
                            #[cfg(target_os = "windows")]
                            { new_config.windows.hide_ingame_ui_hotkey_bind = raw; }
                            #[cfg(target_os = "android")]
                            { new_config.android.hide_ingame_ui_hotkey_bind = raw; }
                            crate::core::gui::save_and_reload_config(new_config);
                        })));
                    });
                }
            });
        });
        ConfigEditor::space(ui, 4.0);
    }

    ConfigEditor::list_tile_switch(ui, t!("config_editor.race_playback_slider"), &mut config.race_playback_slider, true);
    if config.race_playback_slider {
        ConfigEditor::list_tile_switch(ui, t!("config_editor.race_playback_slider_always"), &mut config.race_playback_slider_always, true);
    }
    ConfigEditor::list_tile_switch(ui, t!("config_editor.race_playback_button"), &mut config.race_playback_button, true);
    ConfigEditor::list_tile_switch(ui, t!("config_editor.race_playback_key_enable"), &mut config.race_playback_key_enable, true);

    if config.race_playback_key_enable && !ConfigEditor::row_filtered(&t!("config_editor.race_playback_key")) {
        ConfigEditor::maybe_draw_category_header(ui);
        #[cfg(target_os = "windows")]
        let key_label = crate::windows::utils::vk_to_display_label(config.windows.race_playback_key);
        #[cfg(target_os = "android")]
        let key_label = crate::android::gui_impl::keymap::keycode_display_label(config.android.race_playback_key);
        let secondary_container    = egui_material3::theme::get_global_color("secondaryContainer");
        let on_secondary_container = egui_material3::theme::get_global_color("onSecondaryContainer");
        let key_galley = ui.painter().layout_no_wrap(
            key_label.to_string(),
            ui.style().text_styles[&egui::TextStyle::Body].clone(),
            egui::Color32::WHITE,
        );
        let chip_w = key_galley.size().x + 16.0;

        ui.add(egui::Label::new(t!("config_editor.race_playback_key")).wrap());
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
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
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add(MaterialButton::outlined(t!("bind_key"))).clicked() {
                    thread::spawn(|| {
                        let Some(gui_mutex) = Gui::instance() else { return };
                        let mut gui = gui_mutex.lock().unwrap_or_else(|e| e.into_inner());
                        gui.show_window(Box::new(crate::core::gui::windows::SetKeybindWindow::new(|result| {
                            let Some(raw) = result else { return };
                            let hachimi = crate::core::Hachimi::instance();
                            let mut new_config = hachimi.config.load().as_ref().clone();
                            #[cfg(target_os = "windows")]
                            { new_config.windows.race_playback_key = raw; }
                            #[cfg(target_os = "android")]
                            { new_config.android.race_playback_key = raw; }
                            crate::core::gui::save_and_reload_config(new_config);
                        })));
                    });
                }
            });
        });
        ConfigEditor::space(ui, 4.0);
    }

    ConfigEditor::list_tile_switch(ui, t!("config_editor.race_director_enabled"), &mut config.race_director.enabled, true);
    if config.race_director.enabled {
        ConfigEditor::list_tile_switch(ui, t!("config_editor.race_director_show_tower"), &mut config.race_director.show_tower, true);
        ConfigEditor::list_tile_switch(ui, t!("config_editor.race_director_show_followed"), &mut config.race_director.show_followed, true);
        ConfigEditor::list_tile_switch(ui, t!("config_editor.race_director_show_winprob"), &mut config.race_director.show_winprob, true);
        ConfigEditor::list_tile_switch_described(
            ui, t!("config_editor.race_director_show_predicted"),
            &mut config.race_director.show_predicted, true,
            t!("config_editor.race_director_show_predicted_desc"),
        );
        ConfigEditor::list_tile_slider(ui, t!("config_editor.race_director_opacity"), &mut config.race_director.opacity, 0.1..=1.0, 0.05, 2);
        if ConfigEditor::list_tile_action_button(ui, t!("config_editor.race_director_reset_windows"), t!("config_editor.race_director_reset")) {
            crate::core::gui::race_director_hud::reset_window_positions();
        }
    }

    ConfigEditor::list_tile_switch(ui, t!("config_editor.live_slider_always_show"), &mut config.live_slider_always_show, true);
    ConfigEditor::list_tile_switch(ui, t!("config_editor.live_playback_loop"), &mut config.live_playback_loop, true);
    ConfigEditor::list_tile_switch(ui, t!("config_editor.trainer_live_landscape"), &mut config.trainer_live_landscape, true);
    ConfigEditor::list_tile_switch(ui, t!("config_editor.champions_live_show_text"), &mut config.champions_live_show_text, true);

    if config.champions_live_show_text {
        let choices: Vec<(i32, &str)> = editor.champions_resources.iter().enumerate()
            .map(|(i, n)| ((i + 1) as i32, n.as_str())).collect();
        ConfigEditor::list_tile_combo(ui, t!("config_editor.champions_live_resource_id"),
            "champions_live_resource_id", &mut config.champions_live_resource_id, &choices);

        ConfigEditor::list_tile_number(ui, t!("config_editor.champions_live_year"),
            &mut config.champions_live_year, 2021..=editor.champions_live_max_year, 1.0, None);
    }

    ConfigEditor::list_tile_switch(ui, t!("config_editor.captions"), &mut config.caption.caption_enable, true);
    if config.caption.caption_enable {
        let mut font_size_f = config.caption.caption_font_size as f32;
        if ConfigEditor::list_tile_slider(ui, t!("config_editor.caption_font_size"), &mut font_size_f, 10.0..=128.0, 1.0, 0).changed() {
            config.caption.caption_font_size = font_size_f as i32;
        }
        ConfigEditor::list_tile_slider(ui, t!("config_editor.caption_pos_x"), &mut config.caption.caption_pos_x, -10.0..=10.0, 0.1, 1);
        ConfigEditor::list_tile_slider(ui, t!("config_editor.caption_pos_y"), &mut config.caption.caption_pos_y, -10.0..=10.0, 0.1, 1);
        ConfigEditor::list_tile_slider(ui, t!("config_editor.caption_bg_alpha"), &mut config.caption.caption_bg_alpha, 0.0..=1.0, 0.1, 2);

        ConfigEditor::list_tile_switch_described(
            ui, t!("config_editor.caption_fallback_enable"),
            &mut config.caption.caption_fallback_enable, true,
            t!("config_editor.caption_fallback_tooltip"),
        );

        if config.caption.caption_fallback_enable {
            let mut line_count = config.caption.caption_lines_char_count as f32;
            if ConfigEditor::list_tile_slider(ui, t!("config_editor.caption_lines_char_count"), &mut line_count, 10.0..=100.0, 1.0, 0).changed() {
                config.caption.caption_lines_char_count = line_count.round() as i32;
            }
        }

        ConfigEditor::list_tile_switch(ui, t!("config_editor.caption_show_logging"), &mut config.caption.caption_show_log_enable, true);
        ConfigEditor::list_tile_switch(ui, t!("config_editor.caption_format_logging"), &mut config.caption.caption_format_log_enable, true);

        // Caption string selects
        ConfigEditor::list_tile_string_select(ui, t!("config_editor.caption_color"),
            "caption_color", &mut config.caption.caption_color, &editor.font_color_options.clone());
        ConfigEditor::list_tile_string_select(ui, t!("config_editor.caption_outline_size"),
            "caption_outline_size", &mut config.caption.caption_outline_size, &editor.outline_size_options.clone());
        ConfigEditor::list_tile_string_select(ui, t!("config_editor.caption_outline_color"),
            "caption_outline_color", &mut config.caption.caption_outline_color, &editor.outline_color_options.clone());
    }
}
