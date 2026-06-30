use crate::core::gui::*;

use egui_material3::*;
use rust_i18n::t;

use crate::core::*;
use std::sync::Arc;


pub struct FirstTimeSetupWindow {
    id: egui::Id,
    meta_index_url: String,
    config: hachimi::Config,
    index_request: Arc<AsyncRequest<Vec<RepoInfo>>>,
    current_page: usize,
    current_tl_repo: Option<String>,
    current_tl_repo_mod: Option<String>,
    has_auto_selected: bool,
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
        }
    }
}


impl AppWindow for FirstTimeSetupWindow {
    fn run(&mut self, ctx: &egui::Context) -> bool {
        let mut open = true;
        let mut page_open = true;

        new_window(ctx, self.id, t!("first_time_setup.title"))
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
                    3,
                    allow_next,
                    |ui, i| {
                        match i {
                            0 => {
                                ui.heading(t!("first_time_setup.welcome_heading"));
                                ui.separator();
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
                                #[cfg(target_os = "android")]
                                handle_android_keyboard(&res, &mut self.meta_index_url);
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
                                ui.heading(t!("first_time_setup.translation_repo_heading"));
                                ui.separator();
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

                                        egui::ScrollArea::vertical().show(ui, |ui| {
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
                                ui.heading(t!("first_time_setup.complete_heading"));
                                ui.separator();
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
}




