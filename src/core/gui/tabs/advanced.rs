use crate::core::gui::config::ConfigEditor;
use crate::core::gui::utils::*;
use crate::core::gui::Gui;
use crate::core::gui::dialogs::SimpleOkDialog;
#[allow(unused_imports)]
use crate::core::gui::windows::SetKeybindWindow;
#[allow(unused_imports)]
use crate::core::gui::save_and_reload_config;
use crate::core::hachimi;
#[allow(unused_imports)]
use crate::core::Hachimi;
use egui_material3::*;
use egui_material3::theme::get_global_color;
use rust_i18n::t;
use std::thread;


#[allow(unused_variables)]
pub fn render(editor: &ConfigEditor, config: &mut crate::core::hachimi::Config, ui: &mut egui::Ui) {
    let on_surface_variant = get_global_color("onSurfaceVariant");
    ui.style_mut().visuals.override_text_color = Some(on_surface_variant);

    // ── Advanced ──────────────────────────────────────────────────────────────
    section_heading(ui, t!("config_editor.advanced_settings_heading"));
    ui.add_space(4.0);

    ConfigEditor::list_tile_switch(ui, t!("config_editor.enable_file_logging"), &mut config.enable_file_logging, true);
    ConfigEditor::list_tile_switch(ui, t!("config_editor.enable_ipc"),           &mut config.enable_ipc,           true);
    ConfigEditor::list_tile_switch(ui, t!("config_editor.ipc_listen_all"),        &mut config.ipc_listen_all,       true);
    ConfigEditor::list_tile_switch(ui, t!("config_editor.ipv4_only"),             &mut config.ipv4_only,            true);

    // Meta index URL
    {
        ui.add(egui::Label::new(t!("config_editor.meta_index_url")).wrap());
        // Pin the content width before rendering so focus-state changes
        // (stroke width ±1px) don't shift the layout of surrounding rows.
        ui.set_max_width(ui.available_width());
        let res = ui.add(
            MaterialTextField::filled(&mut config.meta_index_url)
                .lock_focus(true),
        );
        if res.lost_focus() && config.meta_index_url.trim().is_empty() {
            config.meta_index_url = hachimi::Config::default().meta_index_url;
        }
        #[cfg(target_os = "android")]
        handle_android_keyboard(&res, &mut config.meta_index_url);
        #[cfg(target_os = "windows")]
        if res.has_focus() {
            ui.memory_mut(|mem| mem.set_focus_lock_filter(res.id, egui::EventFilter {
                tab: true, horizontal_arrows: true, vertical_arrows: true, escape: true,
                ..Default::default()
            }));
        }
        ui.add_space(4.0);
    }

    // Localized data directory — explicit width prevents layout shifts
    {
        let avail_w = ui.available_width();
        ui.add(egui::Label::new(t!("config_editor.localized_data_dir")).wrap());
        let mut current_dir = config.localized_data_dir.clone()
            .unwrap_or_else(|| "localized_data".to_string());
        let mut sel = editor.localized_data_dirs.iter().position(|v| v == &current_dir);
        let mut select = MaterialSelect::new(&mut sel)
            .variant(SelectVariant::Outlined)
            .placeholder(&current_dir)
            .width(avail_w)
            .small();
        for (i, label) in editor.localized_data_dirs.iter().enumerate() {
            select = select.option(i, label);
        }
        if ui.add(select).changed() {
            if let Some(i) = sel { current_dir = editor.localized_data_dirs[i].clone(); }
        }
        config.localized_data_dir = Some(current_dir);
        ui.add_space(4.0);
    }

    ConfigEditor::list_tile_switch(ui, t!("config_editor.translator_mode"),             &mut config.translator_mode,             true);
    ConfigEditor::list_tile_switch(ui, t!("config_editor.apply_atlas_workaround"),       &mut config.apply_atlas_workaround,       true);
    ConfigEditor::list_tile_switch(ui, t!("config_editor.disable_outdated_asset_notif"), &mut config.disable_outdated_asset_notif, true);
    ConfigEditor::list_tile_switch(ui, t!("config_editor.replace_to_builtin_font"),      &mut config.replace_to_builtin_font,      true);
    ConfigEditor::list_tile_switch(ui, t!("config_editor.skip_first_time_setup"),        &mut config.skip_first_time_setup,        true);
    ConfigEditor::list_tile_switch(ui, t!("config_editor.lazy_translation_updates"),     &mut config.lazy_translation_updates,     true);
    ConfigEditor::list_tile_switch(ui, t!("config_editor.disable_auto_update_check"),    &mut config.disable_auto_update_check,    true);

    // Translation addon index URL
    {
        let mut url = config.translation_repo_index_mod.clone().unwrap_or_default();
        if ConfigEditor::list_tile_text_field(ui, t!("config_editor.translation_repo_index_mod"), &mut url).changed() {
            config.translation_repo_index_mod = if url.is_empty() { None } else { Some(url) };
        }
    }
    ConfigEditor::list_tile_switch(ui, t!("config_editor.disable_mod_downloads"), &mut config.disable_mod_downloads, true);

    ConfigEditor::list_tile_combo(ui, t!("config_editor.bg_update_mode"), "bg_update_mode",
        &mut config.bg_update_mode, &[
            (hachimi::BgUpdateMode::Disabled, &t!("disabled")),
            (hachimi::BgUpdateMode::Periodic, &t!("config_editor.bg_update_periodic")),
            (hachimi::BgUpdateMode::Silent,   &t!("config_editor.bg_update_silent")),
        ]);

    if config.bg_update_mode != hachimi::BgUpdateMode::Disabled {
        // Use list_tile_number for the interval — label above, number field below
        let mut minutes = (config.bg_update_interval_sec / 60) as i32;
        let prev_minutes = minutes;
        ConfigEditor::list_tile_number(ui, t!("config_editor.bg_update_interval"),
            &mut minutes, 1..=10080, 1.0, Some("min"));
        if minutes != prev_minutes {
            config.bg_update_interval_sec = (minutes as u64) * 60;
        }
    }

    ConfigEditor::list_tile_switch(ui, t!("config_editor.disable_translations"), &mut config.disable_translations, true);

    #[cfg(target_os = "android")]
    {
        ConfigEditor::list_tile_switch(ui, t!("config_editor.hook_libc_dlopen"), &mut config.android.hook_libc_dlopen, true);
        ConfigEditor::list_tile_switch(ui, t!("config_editor.keep_screen_on"),   &mut config.android.keep_screen_on,   true);
    }

    #[cfg(target_os = "windows")]
    {
        let supports_smtc    = crate::windows::capabilities::supports_smtc();
        let supports_toasts  = crate::windows::capabilities::supports_scheduled_toasts();
        let supports_taskbar = crate::windows::capabilities::supports_taskbar_progress();

        ConfigEditor::list_tile_switch(ui, t!("config_editor.discord_rpc"), &mut config.windows.discord_rpc, true);

        // SMTC — disabled with Wine hint when unavailable
        if supports_smtc {
            ConfigEditor::list_tile_switch(ui, t!("config_editor.enable_smtc"), &mut config.windows.enable_smtc, true);
        } else {
            ConfigEditor::list_tile_switch(ui, t!("config_editor.enable_smtc"), &mut config.windows.enable_smtc, false);
            ConfigEditor::list_tile_hint(ui, t!("config_editor.unavailable_wine_proton"));
        }

        // Notification settings — disabled with Wine hint when unavailable
        for (label_key, val) in [
            ("config_editor.notification_tp",    &mut config.notification_tp    as &mut bool),
            ("config_editor.notification_rp",    &mut config.notification_rp),
            ("config_editor.notification_jobs",  &mut config.notification_jobs),
        ] {
            if supports_toasts {
                ConfigEditor::list_tile_switch(ui, t!(label_key), val, true);
            } else {
                ConfigEditor::list_tile_switch(ui, t!(label_key), val, false);
                ConfigEditor::list_tile_hint(ui, t!("config_editor.unavailable_wine_proton"));
            }
        }

        // Taskbar progress — disabled with Wine hint when unavailable
        for (label_key, val) in [
            ("config_editor.taskbar_show_progress_on_download",    &mut config.windows.taskbar_show_progress_on_download   as &mut bool),
            ("config_editor.taskbar_show_progress_on_connecting",  &mut config.windows.taskbar_show_progress_on_connecting),
        ] {
            if supports_taskbar {
                ConfigEditor::list_tile_switch(ui, t!(label_key), val, true);
            } else {
                ConfigEditor::list_tile_switch(ui, t!(label_key), val, false);
                ConfigEditor::list_tile_hint(ui, t!("config_editor.unavailable_wine_proton"));
            }
        }

        // Hide-ingame-UI hotkey bind — interactive rebind row
        // Same chip+button pattern as the menu hotkey row in general.rs.
        {
            let key_label = crate::windows::utils::vk_to_display_label(config.windows.hide_ingame_ui_hotkey_bind);
            let secondary_container    = get_global_color("secondaryContainer");
            let on_secondary_container = get_global_color("onSecondaryContainer");
            let key_galley = ui.painter().layout_no_wrap(
                key_label.to_string(),
                ui.style().text_styles[&egui::TextStyle::Body].clone(),
                egui::Color32::WHITE,
            );
            let chip_w = key_galley.size().x + 16.0;

            ui.add(egui::Label::new(t!("config_editor.hide_ingame_ui_hotkey_bind")).wrap());
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                // Key chip
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
                // "Bind" button
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add(MaterialButton::outlined(t!("bind_key"))).clicked() {
                        thread::spawn(|| {
                            let Some(gui_mutex) = Gui::instance() else { return };
                            let mut gui = gui_mutex.lock().unwrap_or_else(|e| e.into_inner());
                            gui.show_window(Box::new(SetKeybindWindow::new(|result| {
                                let Some(raw) = result else { return };
                                let hachimi = Hachimi::instance();
                                let mut new_config = hachimi.config.load().as_ref().clone();
                                #[cfg(target_os = "windows")]
                                { new_config.windows.hide_ingame_ui_hotkey_bind = raw; }
                                #[cfg(target_os = "android")]
                                { new_config.android.hide_ingame_ui_hotkey_bind = raw; }
                                save_and_reload_config(new_config);
                            })));
                        });
                    }
                });
            });
            ui.add_space(4.0);
        }

        ConfigEditor::list_tile_switch(ui, t!("config_editor.ui_loading_show_orientation_guide"),
            &mut config.windows.ui_loading_show_orientation_guide, true);

        ConfigEditor::list_tile_text_field(ui, t!("config_editor.custom_title_name"), {
            // Option<String> shim — read/write through a local String
            let _ = config.custom_title_name.get_or_insert_with(String::new);
            config.custom_title_name.as_mut().unwrap()
        });
        if config.custom_title_name.as_deref() == Some("") {
            config.custom_title_name = None;
        }

        // Full-screen resolution — horizontal layout (W x H or H x W based on orientation)
        let is_landscape = ui.ctx().input(|i| i.viewport_rect().width() > i.viewport_rect().height());
        let label_text = if is_landscape {
            format!("{} ({})", t!("config_editor.full_screen_res"), t!("config_editor.landscape"))
        } else {
            format!("{} ({})", t!("config_editor.full_screen_res"), t!("config_editor.portrait"))
        };
        ui.add(egui::Label::new(label_text).wrap());

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;

            let scale = crate::core::gui::utils::get_scale(ui.ctx());
            let number_w = 48.0 * scale;

            if is_landscape {
                let (rect, _) = ui.allocate_exact_size(egui::vec2(number_w, 32.0), egui::Sense::hover());
                let w_field = ui.put(
                    rect,
                    MaterialNumberField::filled(&mut config.windows.full_screen_res.width)
                        .range(0..=7680)
                        .decimals(0),
                );
                ui.label("W px");
                let (rect2, _) = ui.allocate_exact_size(egui::vec2(number_w, 32.0), egui::Sense::hover());
                let h_field = ui.put(
                    rect2,
                    MaterialNumberField::filled(&mut config.windows.full_screen_res.height)
                        .range(0..=4320)
                        .decimals(0),
                );
                ui.label("H px");

                #[cfg(target_os = "android")]
                {
                    crate::core::gui::utils::handle_android_keyboard(&w_field, &mut config.windows.full_screen_res.width);
                    crate::core::gui::utils::handle_android_keyboard(&h_field, &mut config.windows.full_screen_res.height);
                }
            } else {
                let (rect, _) = ui.allocate_exact_size(egui::vec2(number_w, 32.0), egui::Sense::hover());
                let h_field = ui.put(
                    rect,
                    MaterialNumberField::filled(&mut config.windows.full_screen_res.height)
                        .range(0..=4320)
                        .decimals(0),
                );
                ui.label("H px");
                let (rect2, _) = ui.allocate_exact_size(egui::vec2(number_w, 32.0), egui::Sense::hover());
                let w_field = ui.put(
                    rect2,
                    MaterialNumberField::filled(&mut config.windows.full_screen_res.width)
                        .range(0..=7680)
                        .decimals(0),
                );
                ui.label("W px");

                #[cfg(target_os = "android")]
                {
                    crate::core::gui::utils::handle_android_keyboard(&h_field, &mut config.windows.full_screen_res.height);
                    crate::core::gui::utils::handle_android_keyboard(&w_field, &mut config.windows.full_screen_res.width);
                }
            }
        });
        ui.add_space(4.0);
    }

    ConfigEditor::list_tile_switch(ui, t!("config_editor.hide_now_loading"), &mut config.hide_now_loading, true);

    // ── Experimental ──────────────────────────────────────────────────────────
    ui.add_space(4.0);
    section_heading(ui, t!("config_editor.experimental_settings_heading"));
    ui.add_space(4.0);

    if ConfigEditor::list_tile_switch(ui, t!("config_editor.auto_translate_stories"), &mut config.auto_translate_stories, true)
        .clicked() && config.auto_translate_stories
    {
        thread::spawn(|| {
            Gui::instance().unwrap().lock().unwrap_or_else(|e| e.into_inner())
                .show_window(Box::new(SimpleOkDialog::new(
                    &t!("warning"), &t!("config_editor.auto_tl_warning"), || {},
                )));
        });
    }

    // Sugoi URL
    {
        let mut url = config.sugoi_url.clone().unwrap_or_default();
        if ConfigEditor::list_tile_text_field(ui, t!("config_editor.sugoi_url"), &mut url).changed() {
            config.sugoi_url = if url.is_empty() { None } else { Some(url) };
        }
    }

    if ConfigEditor::list_tile_switch(ui, t!("config_editor.auto_translate_ui"), &mut config.auto_translate_localize, true)
        .clicked() && config.auto_translate_localize
    {
        thread::spawn(|| {
            Gui::instance().unwrap().lock().unwrap_or_else(|e| e.into_inner())
                .show_window(Box::new(SimpleOkDialog::new(
                    &t!("warning"), &t!("config_editor.auto_tl_warning"), || {},
                )));
        });
    }

    ConfigEditor::list_tile_switch(ui, t!("config_editor.unlock_live_chara"),  &mut config.unlock_live_chara,  true);
    ConfigEditor::list_tile_switch(ui, t!("config_editor.msgpack_notifier"),   &mut config.msgpack_notifier,   true);

    if config.msgpack_notifier {
        ConfigEditor::list_tile_text_field(ui, t!("config_editor.msgpack_notifier_host"), &mut config.msgpack_notifier_host);
        ConfigEditor::list_tile_switch(ui, t!("config_editor.msgpack_notifier_request"), &mut config.msgpack_notifier_request, true);
        ConfigEditor::list_tile_number(ui, t!("config_editor.msgpack_notifier_connection_timeout_ms"),
            &mut config.msgpack_notifier_connection_timeout_ms, 100..=30000, 100.0, Some("ms"));
        ConfigEditor::list_tile_switch(ui, t!("config_editor.msgpack_notifier_print_error"), &mut config.msgpack_notifier_print_error, true);
    }

    ConfigEditor::list_tile_switch(ui, t!("config_editor.dump_msgpack"),         &mut config.dump_msgpack,         true);
    ConfigEditor::list_tile_switch(ui, t!("config_editor.dump_msgpack_request"), &mut config.dump_msgpack_request, true);

    // ── Developer ─────────────────────────────────────────────────────────────
    ui.add_space(4.0);
    section_heading(ui, t!("config_editor.developer_settings_heading"));
    ui.add_space(4.0);

    ConfigEditor::list_tile_switch(ui, t!("config_editor.debug_mode"), &mut config.debug_mode, true);

    if ConfigEditor::list_tile_switch(ui, t!("config_editor.text_debug"), &mut config.text_debug, true)
        .clicked() && !config.text_debug
    {
        config.text_log = false; config.text_property_dump = false;
        config.text_localize_dump = false; config.text_position_debug = false;
        config.text_path_debug = false;
    }

    if config.text_debug {
        ConfigEditor::list_tile_switch(ui, format!("    - {}", t!("config_editor.text_log")),            &mut config.text_log,            true);
        ConfigEditor::list_tile_switch(ui, format!("    - {}", t!("config_editor.text_property_dump")),  &mut config.text_property_dump,  true);
        ConfigEditor::list_tile_switch(ui, format!("    - {}", t!("config_editor.text_localize_dump")),  &mut config.text_localize_dump,  true);
        ConfigEditor::list_tile_switch(ui, format!("    - {}", t!("config_editor.text_position_debug")), &mut config.text_position_debug, true);
        ConfigEditor::list_tile_switch(ui, format!("    - {}", t!("config_editor.text_path_debug")),     &mut config.text_path_debug,     true);
    }
}
