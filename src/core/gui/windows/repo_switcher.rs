use crate::core::gui::*;
use crate::core::hachimi;
use crate::core::http::AsyncRequest;
use crate::core::tl_repo;
use crate::core::tl_repo::RepoInfo;
use crate::core::Hachimi;
use egui_material3::*;
use log::warn;
use rust_i18n::t;
use std::sync::Arc;
use std::thread;

pub struct RepoSwitcherWindow {
    id: egui::Id,
    confirm_remove: Option<(u32, String)>,
}

impl RepoSwitcherWindow {
    pub fn new() -> RepoSwitcherWindow {
        RepoSwitcherWindow {
            id: random_id(),
            confirm_remove: None,
        }
    }

    fn switch_to_repo(id: u32, index: &str) {
        let hachimi = Hachimi::instance();
        let config = hachimi.config.load();
        let mut new_config = (**config).clone();
        new_config.selected_tl_repo_id = Some(id);
        new_config.translation_repo_index = Some(index.to_string());
        drop(config);
        save_and_reload_config(new_config);
    }

    fn remove_repo_async(id: u32) {
        thread::spawn(move || {
            let hachimi = Hachimi::instance();
            let repos_path = hachimi.get_data_path(".tl_repos");
            let mut manager = hachimi.tl_repo_manager.lock().unwrap();
            manager.repos.retain(|r| r.id != id);
            if let Err(e) = manager.save(&repos_path) {
                warn!("Failed to save .tl_repos: {e}");
            }
        });
    }

    fn repo_row(
        ui: &mut egui::Ui,
        repo: &tl_repo::RepoEntry,
        selected_id: &mut Option<u32>,
        confirm_remove: &mut Option<(u32, String)>,
        scale: f32,
    ) {
        let is_active = *selected_id == Some(repo.id);
        if confirm_remove.as_ref().map(|(id, _)| *id) == Some(repo.id) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0 * scale;
                if is_active {
                    ui.label(t!("change_translation_repo.cannot_remove_active"));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(MaterialButton::filled(t!("ok"))).clicked() {
                            *confirm_remove = None;
                        }
                    });
                } else {
                    if let Some((_, ref name)) = *confirm_remove {
                        ui.label(t!("change_translation_repo.confirm_remove", name = name));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(MaterialButton::outlined(t!("cancel"))).clicked() {
                            *confirm_remove = None;
                        }
                        if ui.add(MaterialButton::filled(t!("remove"))).clicked() {
                            let (id, _) = confirm_remove.take().unwrap();
                            Self::remove_repo_async(id);
                        }
                    });
                }
            });
            ui.add_space(4.0 * scale);
            return;
        }

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0 * scale;
            let mut remove_clicked = false;
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add(MaterialButton::outlined(t!("remove"))).clicked() {
                    remove_clicked = true;
                }

                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    let radio_resp = ui.add(egui::RadioButton::new(is_active, ""));
                    if radio_resp.clicked() && !is_active {
                        *selected_id = Some(repo.id);
                        Self::switch_to_repo(repo.id, &repo.index);
                    }

                    let label = egui::Label::new(
                        egui::RichText::new(&repo.index)
                    ).truncate();
                    let label_resp = ui.add(label.sense(egui::Sense::click()));
                    if label_resp.clicked() && !is_active {
                        *selected_id = Some(repo.id);
                        Self::switch_to_repo(repo.id, &repo.index);
                    }
                });
            });

            if remove_clicked {
                *confirm_remove = Some((repo.id, repo.index.clone()));
            }
        });
        ui.add_space(4.0 * scale);
    }
}

impl AppWindow for RepoSwitcherWindow {
    fn run(&mut self, ctx: &egui::Context) -> bool {
        let scale = get_scale(ctx);
        let mut open = true;
        let mut open2 = true;

        let hachimi = Hachimi::instance();
        let manager = hachimi.tl_repo_manager.lock().unwrap();
        let current_repo_id = hachimi.config.load().selected_tl_repo_id;
        let mut selected_id = current_repo_id;
        let has_repos = !manager.repos.is_empty();

        new_window(ctx, self.id, t!("change_translation_repo.title"))
            .open(&mut open)
            .show(ctx, |ui| {
                simple_window_layout(
                    ui,
                    self.id,
                    |ui| {
                        let avail_w = ui.available_width();
                        egui::ScrollArea::vertical()
                            .id_salt("repo_switcher_scroll")
                            .auto_shrink([false, true])
                            .max_height(240.0 * scale)
                            .show(ui, |ui| {
                                ui.set_width(avail_w);
                                egui::Frame::NONE
                                    .inner_margin(egui::Margin::symmetric(
                                        (LIST_TILE_PAD_H * scale) as i8,
                                        (4.0 * scale) as i8,
                                    ))
                                    .show(ui, |ui| {
                                        if !has_repos {
                                            ui.vertical_centered(|ui| {
                                                ui.add_space(20.0 * scale);
                                                ui.label(t!("change_translation_repo.no_repos"));
                                                ui.add_space(10.0 * scale);
                                            });
                                        } else {
                                            let active_count = manager.repos.iter().filter(|r| current_repo_id == Some(r.id)).count();
                                            let available_count = manager.repos.iter().filter(|r| current_repo_id != Some(r.id)).count();

                                            if active_count > 0 {
                                                section_heading(ui, t!("change_translation_repo.active"));

                                                for repo in &manager.repos {
                                                    if current_repo_id == Some(repo.id) {
                                                        Self::repo_row(ui, repo, &mut selected_id, &mut self.confirm_remove, scale);
                                                    }
                                                }
                                            }

                                            if available_count > 0 {
                                                section_heading(ui, t!("change_translation_repo.available"));

                                                for repo in &manager.repos {
                                                    if current_repo_id != Some(repo.id) {
                                                        Self::repo_row(ui, repo, &mut selected_id, &mut self.confirm_remove, scale);
                                                    }
                                                }
                                            }
                                        }
                                    });
                                let ime_pad = ime_scroll_padding(ui.ctx());
                                if ime_pad > 0.0 { ui.add_space(ime_pad); }
                            });
                    },
                    |ui| {
                        if ui.add(MaterialButton::outlined(t!("cancel"))).clicked() {
                            open2 = false;
                        }
                        if ui.add(MaterialButton::filled(t!("change_translation_repo.browse_repositories"))).clicked() {
                            thread::spawn(|| {
                                let Some(gui_mutex) = Gui::instance() else {
                                    return;
                                };
                                let mut gui = gui_mutex.lock().unwrap();
                                gui.show_window(Box::new(AddRepoWindow::new()));
                            });
                        }
                    },
                );
            });

        open &= open2;
        open
    }

    fn window_id(&self) -> Option<egui::Id> {
        Some(self.id)
    }
}

pub struct AddRepoWindow {
    id: egui::Id,
    index_request: Arc<AsyncRequest<Vec<RepoInfo>>>,
    config: hachimi::Config,
    current_tl_repo: Option<String>,
    current_tl_repo_mod: Option<String>,
    has_auto_selected: bool,
}

impl AddRepoWindow {
    pub fn new() -> AddRepoWindow {
        let config = (**Hachimi::instance().config.load()).clone();
        AddRepoWindow {
            id: random_id(),
            index_request: Arc::new(tl_repo::new_meta_index_request()),
            config,
            current_tl_repo: None,
            current_tl_repo_mod: None,
            has_auto_selected: false,
        }
    }
}

impl AppWindow for AddRepoWindow {
    fn run(&mut self, ctx: &egui::Context) -> bool {
        let scale = get_scale(ctx);
        let mut open = true;
        let mut open2 = true;

        let builder = egui::UiBuilder::new()
            .id(self.id)
            .layout(egui::Layout::top_down(egui::Align::Center).with_cross_justify(true));

        new_window(ctx, self.id, t!("add_translation_repo.title"))
            .open(&mut open)
            .show(ctx, |ui| {
                ui.scope_builder(builder, |ui| {
                    ui.with_layout(
                        egui::Layout::top_down(egui::Align::Min).with_cross_justify(true),
                        |ui| {
                            let avail_w = ui.available_width();
                            egui::ScrollArea::vertical()
                                .id_salt("add_repo_scroll")
                                .auto_shrink([false, true])
                                .max_height(240.0 * scale)
                                .show(ui, |ui| {
                                    ui.set_width(avail_w);
                                    egui::Frame::NONE
                                        .inner_margin(egui::Margin::symmetric(
                                            (LIST_TILE_PAD_H * scale) as i8,
                                            (4.0 * scale) as i8,
                                        ))
                                        .show(ui, |ui| {
                                            ui.label(t!("add_translation_repo.select_translation_repo"));
                                            ui.add_space(4.0 * scale);

                                            let index_req = self.index_request.clone();
                                            let current_lang_str = self.config.language.locale_str();

                                            async_request_ui_content(
                                                ui,
                                                index_req,
                                                || {
                                                    // Retrigger
                                                },
                                                |ui, repo_list| {
                                                    let hachimi = Hachimi::instance();

                                                    let mut filtered_repos: Vec<_> = repo_list
                                                        .iter()
                                                        .filter(|repo| repo.region == hachimi.game.region)
                                                        .collect();

                                                    if !self.has_auto_selected && self.current_tl_repo.is_none() {
                                                        if let Some(matched) = filtered_repos
                                                            .iter()
                                                            .find(|r| r.is_recommended(current_lang_str))
                                                        {
                                                            self.current_tl_repo = Some(matched.index.clone());
                                                            self.current_tl_repo_mod =
                                                                matched.index_mod.clone();
                                                        }
                                                        self.has_auto_selected = true;
                                                    }

                                                    filtered_repos.sort_by_key(|repo| {
                                                        !repo.is_recommended(current_lang_str)
                                                    });

                                                    if filtered_repos.is_empty() {
                                                        ui.label(t!("first_time_setup.no_compatible_repo"));
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

                                                    for repo in filtered_repos.iter() {
                                                        let has_addon = repo.index_mod.is_some();
                                                        let addon_suffix = if has_addon {
                                                            " (+ addon)"
                                                        } else {
                                                            ""
                                                        };
                                                        if ui
                                                            .add(MaterialRadio::new(
                                                                &mut selected_repo,
                                                                repo.index.clone(),
                                                                format!(
                                                                    "{}{}",
                                                                    repo.name, addon_suffix
                                                                ),
                                                            ))
                                                            .changed()
                                                        {
                                                            self.current_tl_repo_mod =
                                                                repo.index_mod.clone();
                                                        }
                                                        if let Some(short_desc) = &repo.short_desc {
                                                            ui.label(
                                                                egui::RichText::new(short_desc)
                                                                    .small(),
                                                            );
                                                        }
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
                                                },
                                            );
                                        });
                                    let ime_pad = ime_scroll_padding(ui.ctx());
                                    if ime_pad > 0.0 { ui.add_space(ime_pad); }
                                });
                        },
                    );

                    ui.separator();

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                        if ui.add(MaterialButton::outlined(t!("cancel"))).clicked() {
                            open2 = false;
                        }
                        if ui
                            .add_enabled(
                                self.current_tl_repo.is_some(),
                                MaterialButton::filled(t!("save_changes")),
                            )
                            .clicked()
                        {
                            if let Some(index) = self.current_tl_repo.clone() {
                                let hachimi = Hachimi::instance();
                                let id = {
                                    let mut manager =
                                        hachimi.tl_repo_manager.lock().unwrap();
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
                                let config = hachimi.config.load();
                                let mut new_config = (**config).clone();
                                new_config.selected_tl_repo_id = Some(id);
                                new_config.translation_repo_index = Some(index);
                                new_config.translation_repo_index_mod =
                                    self.current_tl_repo_mod.clone();
                                drop(config);
                                save_and_reload_config(new_config);
                            }
                            open2 = false;
                        }
                    });
                });
            });

        open &= open2;
        open
    }

    fn window_id(&self) -> Option<egui::Id> {
        Some(self.id)
    }
}
