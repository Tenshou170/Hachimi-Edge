use crate::core::gui::*;

use egui_material3::{theme::get_global_color, *};
use rust_i18n::t;

use crate::core::*;
use std::sync::{Arc, Mutex};
use crate::core::gui::windows::SetKeybindWindow;

#[cfg(target_os = "windows")]
type RawKeybind = u16;
#[cfg(target_os = "android")]
type RawKeybind = i32;

pub struct FirstTimeSetupWindow {
    id: egui::Id,
    meta_index_url: String,
    config: hachimi::Config,
    index_request: Arc<AsyncRequest<Vec<RepoInfo>>>,
    current_page: usize,
    current_tl_repo: Option<String>,
    current_tl_repo_mod: Option<String>,
    has_auto_selected: bool,
    pending_keybind: Arc<Mutex<Option<RawKeybind>>>,
}


impl FirstTimeSetupWindow {
    pub fn new() -> FirstTimeSetupWindow {
        let config = (**Hachimi::instance().config.load()).clone();
        FirstTimeSetupWindow {
            id: random_id(),
            meta_index_url: config.meta_index_url.clone(),
            config,
            index_request: Arc::new(tl_repo::new_meta_index_request()),
            current_page: 0,
            current_tl_repo: None,
            current_tl_repo_mod: None,
            has_auto_selected: false,
            pending_keybind: Arc::new(Mutex::new(None)),
        }
    }
}


impl AppWindow for FirstTimeSetupWindow {
    fn run(&mut self, ctx: &egui::Context) -> bool {
        let mut open = true;
        let mut page_open = true;

        if let Some(raw) = self.pending_keybind.lock().unwrap().take() {
            #[cfg(target_os = "windows")]
            { self.config.windows.menu_open_key = raw; }
            #[cfg(target_os = "android")]
            { self.config.android.menu_open_key = raw; }
        }

        let mut window = new_window(ctx, self.id.with(self.current_page), t!("first_time_setup.title"));
        if self.current_page == 2 {
            window = window.fixed_size(subwindow_size(ctx));
        }

        window
            .open(&mut open)
            .show(ctx, |ui| {
                let allow_next = match self.current_page {
                    1 => (**self.index_request.result.load())
                        .as_ref()
                        .map_or(false, |r| r.is_ok()),
                    _ => true,
                };

                page_open = paginated_window_layout(
                    ui,
                    self.id,
                    &mut self.current_page,
                    4,
                    allow_next,
                    |ui, i| {
                        match i {
                            0 => {
                                ui.add(egui::Label::new(
                                    egui::RichText::new(t!("first_time_setup.welcome_heading"))
                                        .size(16.0)
                                        .strong()
                                        .color(get_global_color("onSurface")),
                                ));
                                ui.add_space(8.0);
                                let mut language = self.config.language;
                                let lang_changed = ConfigEditor::list_tile_combo(
                                    ui,
                                    t!("config_editor.language"),
                                    "language",
                                    &mut language,
                                    Language::CHOICES,
                                );
                                if lang_changed {
                                    self.config.language = language;
                                    save_and_reload_config(self.config.clone());
                                    self.current_tl_repo = None;
                                }

                                let res = ConfigEditor::list_tile_text_field(
                                    ui,
                                    t!("config_editor.meta_index_url"),
                                    &mut self.meta_index_url,
                                );
                                #[cfg(target_os = "windows")]
                                if res.has_focus() {
                                    ui.memory_mut(|mem| {
                                        mem.set_focus_lock_filter(
                                            res.id,
                                            egui::EventFilter {
                                                tab: true,
                                                horizontal_arrows: true,
                                                vertical_arrows: true,
                                                escape: true,
                                                ..Default::default()
                                            },
                                        )
                                    });
                                }

                                if res.lost_focus() {
                                    if self.meta_index_url.trim().is_empty() {
                                        self.meta_index_url =
                                            hachimi::Config::default().meta_index_url;
                                    }

                                    if self.meta_index_url != self.config.meta_index_url {
                                        self.config.meta_index_url =
                                            self.meta_index_url.clone();
                                        save_and_reload_config(self.config.clone());
                                        self.index_request =
                                            Arc::new(tl_repo::new_meta_index_request());
                                    }
                                }
                                ui.label(t!("first_time_setup.welcome_content"));
                            }
                            1 => {
                                ui.add(egui::Label::new(
                                    egui::RichText::new(t!("first_time_setup.translation_repo_heading"))
                                        .size(16.0)
                                        .strong()
                                        .color(get_global_color("onSurface")),
                                ));
                                ui.add_space(8.0);
                                ui.label(t!("first_time_setup.select_translation_repo"));
                                ui.add_space(4.0);

                                async_request_ui_content(
                                    ui,
                                    self.index_request.clone(),
                                    || {
                                        self.index_request =
                                            Arc::new(tl_repo::new_meta_index_request());
                                    },
                                    |ui, repo_list| {
                                        let hachimi = Hachimi::instance();
                                        let current_lang_str = self.config.language.locale_str();

                                        let mut filtered_repos: Vec<_> = repo_list
                                            .iter()
                                            .filter(|repo| repo.region == hachimi.game.region)
                                            .collect();

                                        if !self.has_auto_selected && self.current_tl_repo.is_none()
                                        {
                                            if let Some(matched) = filtered_repos
                                                .iter()
                                                .find(|r| r.is_recommended(current_lang_str))
                                            {
                                                self.current_tl_repo = Some(matched.index.clone());
                                                self.current_tl_repo_mod = matched.index_mod.clone();
                                            }
                                            self.has_auto_selected = true;
                                        }

                                        filtered_repos.sort_by_key(|repo| {
                                            !repo.is_recommended(current_lang_str)
                                        });

                                        let scale = get_scale(ui.ctx());
                                        egui::ScrollArea::vertical()
                                            .id_salt("setup_repo_scroll")
                                            .max_height(240.0 * scale)
                                            .show(ui, |ui| {
                                                egui::Frame::NONE
                                                    .inner_margin(egui::Margin::symmetric(8, 0))
                                                    .show(ui, |ui| {
                                                        if filtered_repos.is_empty() {
                                                            ui.label(t!(
                                                                "first_time_setup.no_compatible_repo"
                                                            ));
                                                            return;
                                                        }
                                                        let mut selected_repo = Some(
                                                            self.current_tl_repo
                                                                .clone()
                                                                .unwrap_or_default(),
                                                        );
                                                        ui.add(MaterialRadio::new(
                                                            &mut selected_repo,
                                                            String::new(),
                                                            t!("first_time_setup.skip_translation"),
                                                        ));

                                                        let mut last_section: Option<bool> = None;

                                                        for repo in filtered_repos.iter() {
                                                            let is_matched =
                                                                repo.is_recommended(current_lang_str);
                                                            let is_selected =
                                                                self.current_tl_repo.as_ref()
                                                                    == Some(&repo.index);

                                                            // Add separator before switching from matched to unmatched
                                                            if let Some(prev_matched) = last_section {
                                                                if prev_matched != is_matched {
                                                                    ui.separator();
                                                                }
                                                            }
                                            
                                                            // Build label with addon indicator
                                                            let has_addon = repo.index_mod.is_some();
                                                            let addon_suffix = if has_addon { " (+ addon)" } else { "" };

                                                            // Visual indicator for auto-selected matched language repo
                                                            if is_matched && is_selected {
                                                                let repo_label = format!("★ {}{}", repo.name, addon_suffix);
                                                                if ui.add(MaterialRadio::new(
                                                                    &mut selected_repo,
                                                                    repo.index.clone(),
                                                                    repo_label,
                                                                )).changed() {
                                                                    self.current_tl_repo_mod = repo.index_mod.clone();
                                                                }
                                                                if let Some(short_desc) = &repo.short_desc {
                                                                    ui.label(egui::RichText::new(short_desc).small());
                                                                }
                                                            } else {
                                                                let repo_label = format!("{}{}", repo.name, addon_suffix);
                                                                if ui.add(MaterialRadio::new(
                                                                    &mut selected_repo,
                                                                    repo.index.clone(),
                                                                    repo_label,
                                                                )).changed() {
                                                                    self.current_tl_repo_mod = repo.index_mod.clone();
                                                                }
                                                                if let Some(short_desc) = &repo.short_desc {
                                                                    ui.label(egui::RichText::new(short_desc).small());
                                                                }
                                                            }

                                                            last_section = Some(is_matched);
                                                        }
                                                        self.current_tl_repo = if selected_repo
                                                            .as_deref()
                                                            .unwrap_or_default()
                                                            .is_empty()
                                                        {
                                                            None
                                                        } else {
                                                            selected_repo
                                                        };
                                                    });
                                                let ime_pad = ime_scroll_padding(ui.ctx());
                                                if ime_pad > 0.0 { ui.add_space(ime_pad); }
                                            });
                                    },
                                );
                            }
                            2 => {
                                ui.add(egui::Label::new(
                                    egui::RichText::new(t!("first_time_setup.common_settings_heading"))
                                        .size(16.0)
                                        .strong()
                                        .color(get_global_color("onSurface")),
                                 ));
                                ui.add_space(8.0);
                                ui.label(t!("first_time_setup.common_settings_content"));
                                ui.add_space(8.0);

                                let scale = get_scale(ui.ctx());
                                let avail_w = ui.available_width();

                                egui::ScrollArea::vertical()
                                    .id_salt("setup_common_settings_scroll")
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| {
                                        ui.set_width(avail_w);

                                        // 1. Target FPS
                                        ConfigEditor::list_tile_option_slider(
                                            ui,
                                            &t!("config_editor.target_fps"),
                                            &mut self.config.target_fps,
                                            30..=240,
                                        );

                                        // 2. Menu Open Keybind — label top, chip + bind button bottom
                                        //    (matches Config Editor general tab layout)
                                        #[cfg(target_os = "windows")]
                                        let key_label = crate::windows::utils::vk_to_display_label(self.config.windows.menu_open_key);
                                        #[cfg(target_os = "android")]
                                        let key_label = crate::android::gui_impl::keymap::keycode_display_label(self.config.android.menu_open_key);

                                        let secondary_container    = get_global_color("secondaryContainer");
                                        let on_secondary_container = get_global_color("onSecondaryContainer");

                                        let key_galley = ui.painter().layout_no_wrap(
                                            key_label.to_string(),
                                            ui.style().text_styles[&egui::TextStyle::Body].clone(),
                                            egui::Color32::WHITE,
                                        );
                                        let chip_w = key_galley.size().x + 16.0;

                                        // Top line: setting name
                                        ui.add(egui::Label::new(t!("config_editor.menu_open_key")).wrap());
                                        // Bottom line: chip left, "Set" button right
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
                                                if ui.add(MaterialButton::outlined(t!("config_editor.menu_open_key_set"))).clicked() {
                                                    let keybind_slot = self.pending_keybind.clone();
                                                    std::thread::spawn(move || {
                                                        let Some(gui_mutex) = Gui::instance() else { return };
                                                        let mut gui = gui_mutex.lock().unwrap_or_else(|e| e.into_inner());
                                                        gui.show_window(Box::new(SetKeybindWindow::new(move |result| {
                                                            let Some(raw) = result else { return };
                                                            let hachimi = Hachimi::instance();
                                                            let mut new_config = hachimi.config.load().as_ref().clone();
                                                            #[cfg(target_os = "windows")]
                                                            { new_config.windows.menu_open_key = raw; }
                                                            #[cfg(target_os = "android")]
                                                            { new_config.android.menu_open_key = raw; }
                                                            save_and_reload_config(new_config);
                                                            *keybind_slot.lock().unwrap() = Some(raw);
                                                        })));
                                                    });
                                                }
                                            });
                                        });
                                        ConfigEditor::space(ui, scale * 8.0);

                                        // 3. Captions / Subtitles Toggle
                                        ConfigEditor::list_tile_switch(
                                            ui,
                                            t!("config_editor.captions"),
                                            &mut self.config.caption.caption_enable,
                                            true,
                                        );

                                        // 4. Disable Skill Name Translation
                                        ConfigEditor::list_tile_switch(
                                            ui,
                                            t!("config_editor.disable_skill_name_translation"),
                                            &mut self.config.disable_skill_name_translation,
                                            true,
                                        );

                                        // 5. Disable Tap Effect
                                        ConfigEditor::list_tile_switch(
                                            ui,
                                            t!("config_editor.disable_tap_effect"),
                                            &mut self.config.disable_tap_effect,
                                            true,
                                        );

                                        // 6. Discord RPC (Windows only)
                                        #[cfg(target_os = "windows")]
                                        {
                                            ConfigEditor::list_tile_switch(
                                                ui,
                                                t!("config_editor.discord_rpc"),
                                                &mut self.config.windows.discord_rpc,
                                                true,
                                            );
                                        }

                                        let ime_pad = ime_scroll_padding(ui.ctx());
                                        if ime_pad > 0.0 { ui.add_space(ime_pad); }
                                    });
                            }
                            3 => {
                                ui.add(egui::Label::new(
                                    egui::RichText::new(t!("first_time_setup.complete_heading"))
                                        .size(16.0)
                                        .strong()
                                        .color(get_global_color("onSurface")),
                                ));
                                ui.add_space(8.0);
                                ui.label(t!("first_time_setup.complete_content"));
                            }
                            _ => {}
                        }
                    },
                );
            });

        let open_res = open && page_open;
        if !open_res {
            self.config.skip_first_time_setup = true;

            if !page_open {
                self.config.translation_repo_index = self.current_tl_repo.clone();
                self.config.translation_repo_index_mod = self.current_tl_repo_mod.clone();

                // Register the chosen repo so the registry + active dir are set up
                // immediately, not only after the first update check.
                if let Some(index) = self.current_tl_repo.clone() {
                    let hachimi = Hachimi::instance();
                    let id = {
                        let mut manager = hachimi.tl_repo_manager.lock().unwrap();
                        let repos_path = hachimi.get_data_path(".tl_repos");
                        match manager.find_by_index(&index) {
                            Some(existing) => existing,
                            None => {
                                let new_id = manager.add(index.clone());
                                if let Err(e) = manager.save(&repos_path) {
                                    warn!("Failed to save .tl_repos: {e}");
                                }
                                new_id
                            }
                        }
                    };
                    self.config.selected_tl_repo_id = Some(id);
                }
            }

            save_and_reload_config(self.config.clone());

            if !page_open {
                Hachimi::instance()
                    .tl_updater
                    .clone()
                    .check_for_updates(false, false);
            }
        }

        open_res
    }

    fn window_id(&self) -> Option<egui::Id> {
        Some(self.id.with(self.current_page))
    }
}




