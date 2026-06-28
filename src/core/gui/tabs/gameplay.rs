use crate::core::gui::config::ConfigEditor;
use crate::core::gui::Gui;
use crate::core::gui::dialogs::SimpleOkDialog;
use crate::core::gui::windows::live_vocals::LiveVocalsSwapWindow;
use crate::core::utils::get_localized_string;
#[allow(unused_imports)]
use egui_material3::*;
use rust_i18n::t;
use std::thread;
use crate::il2cpp::hook::umamusume::CySpringController::SpringUpdateMode;
use crate::il2cpp::hook::umamusume::TimeUtil::BgSeason;

#[allow(unused_variables)]
pub fn render(editor: &ConfigEditor, config: &mut crate::core::hachimi::Config, ui: &mut egui::Ui) {
    let on_surface_variant = get_global_color("onSurfaceVariant");
    ui.style_mut().visuals.override_text_color = Some(on_surface_variant);

    ConfigEditor::list_tile_combo(ui, t!("config_editor.physics_update_mode"), "physics_update_mode",
        &mut config.physics_update_mode, &[
            (None, &t!("default")),
            (SpringUpdateMode::ModeNormal.into(), "ModeNormal"),
            (SpringUpdateMode::Mode60FPS.into(), "Mode60FPS"),
            (SpringUpdateMode::SkipFrame.into(), "SkipFrame"),
            (SpringUpdateMode::SkipFramePostAlways.into(), "SkipFramePostAlways"),
        ]);
    ConfigEditor::list_tile_switch(ui, t!("config_editor.cyspring_mono_uncap_frame_scale"), &mut config.cyspring_mono_uncap_frame_scale, true);
    ConfigEditor::list_tile_switch(ui, t!("config_editor.cyspring_disable_native"), &mut config.cyspring_disable_native, true);
    ConfigEditor::list_tile_slider(ui, t!("config_editor.story_choice_auto_select_delay"), &mut config.story_choice_auto_select_delay, 0.1..=10.0, 0.05, 2);
    ConfigEditor::list_tile_slider(ui, t!("config_editor.story_text_speed_multiplier"), &mut config.story_tcps_multiplier, 0.1..=10.0, 0.1, 1);
    ConfigEditor::list_tile_switch(ui, t!("config_editor.force_allow_dynamic_camera"), &mut config.force_allow_dynamic_camera, true);
    ConfigEditor::list_tile_switch(ui, t!("config_editor.live_theater_allow_same_chara"), &mut config.live_theater_allow_same_chara, true);

    if ConfigEditor::list_tile_button(ui, t!("config_editor.live_vocals_swap"), t!("open")) {
        thread::spawn(|| {
            Gui::instance().unwrap().lock().unwrap_or_else(|e| e.into_inner())
                .show_window(Box::new(LiveVocalsSwapWindow::new()));
        });
    }

    ConfigEditor::list_tile_switch(ui, t!("config_editor.skill_info_dialog"), &mut config.skill_info_dialog, true);
    ConfigEditor::list_tile_combo(ui, t!("config_editor.homescreen_bgseason"), "homescreen_bgseason",
        &mut config.homescreen_bgseason, &[
            (BgSeason::None, &t!("default")),
            (BgSeason::Spring,        &get_localized_string("Common0108").as_str()),
            (BgSeason::Summer,        &get_localized_string("Common0109").as_str()),
            (BgSeason::Fall,          &get_localized_string("Common0110").as_str()),
            (BgSeason::Winter,        &get_localized_string("Common0111").as_str()),
            (BgSeason::CherryBlossom, &get_localized_string("Common0112").as_str()),
        ]);
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

    ConfigEditor::list_tile_switch(ui, t!("config_editor.live_slider_always_show"), &mut config.live_slider_always_show, true);
    ConfigEditor::list_tile_switch(ui, t!("config_editor.live_playback_loop"), &mut config.live_playback_loop, true);
    ConfigEditor::list_tile_switch(ui, t!("config_editor.champions_live_show_text"), &mut config.champions_live_show_text, true);

    if config.champions_live_show_text {
        let choices: Vec<(i32, &str)> = editor.champions_resources.iter().enumerate()
            .map(|(i, n)| ((i + 1) as i32, n.as_str())).collect();
        ConfigEditor::list_tile_combo(ui, t!("config_editor.champions_live_resource_id"),
            "champions_live_resource_id", &mut config.champions_live_resource_id, &choices);

        ConfigEditor::list_tile_number(ui, t!("config_editor.champions_live_year"),
            &mut config.champions_live_year, 2021..=2030, 1.0, None);
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

        ConfigEditor::list_tile_switch(ui, t!("config_editor.caption_fallback_enable"), &mut config.caption.caption_fallback_enable, true);
        ConfigEditor::list_tile_hint(ui, t!("config_editor.caption_fallback_tooltip"));

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
