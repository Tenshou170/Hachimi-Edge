use std::sync::atomic::AtomicI32;

use crate::core::gui::*;

use egui_material3::*;
use rust_i18n::t;

use crate::core::*;

static LVSW_LAST_CONCERT_ID: AtomicI32 = AtomicI32::new(0);

pub struct LiveVocalsSwapWindow {
    id: egui::Id,
    config: hachimi::Config,
    chara_choices: Vec<(i32, String)>,
    concert_choices: Vec<(i32, String)>,
    selected_concert: i32,
    search_term: String,
}

impl LiveVocalsSwapWindow {
    pub fn new() -> LiveVocalsSwapWindow {
        let hachimi = Hachimi::instance();
        hachimi.init_character_data();
        hachimi.init_live_data();

        let mut chara_choices: Vec<(i32, String)> = Vec::new();
        chara_choices.push((0, t!("default").into_owned()));

        let data = hachimi.chara_data.load();
        for &id in &data.chara_ids {
            chara_choices.push((id, data.get_name(id)));
        }
        chara_choices.sort_by_key(|choice| choice.0);

        let mut concert_choices: Vec<(i32, String)> = Vec::new();
        concert_choices.push((0, t!("config_editor.live_vocals_swap_all_songs").into_owned()));
        let live_data = hachimi.live_data.load();
        for &id in &live_data.live_ids {
            concert_choices.push((id, live_data.get_name(id)));
        }

        let stored_concert = LVSW_LAST_CONCERT_ID.load(std::sync::atomic::Ordering::Relaxed);
        let selected_concert = if concert_choices.iter().any(|&(id, _)| id == stored_concert) {
            stored_concert
        } else {
            0
        };

        LiveVocalsSwapWindow {
            id: random_id(),
            config: (**hachimi.config.load()).clone(),
            chara_choices,
            concert_choices,
            selected_concert,
            search_term: String::new(),
        }
    }
}

impl AppWindow for LiveVocalsSwapWindow {
    fn run(&mut self, ctx: &egui::Context) -> bool {
        let scale = get_scale(ctx);
        let mut open = true;
        let mut open2 = true;
        let mut save_clicked = false;
        let concert_before = self.selected_concert;

        let combo_items: Vec<(i32, &str)> = self
            .chara_choices
            .iter()
            .map(|&(id, ref name)| (id, name.as_str()))
            .collect();
        let concert_items: Vec<(i32, &str)> = self
            .concert_choices
            .iter()
            .map(|&(id, ref name)| (id, name.as_str()))
            .collect();

        new_window(ctx, self.id, t!("config_editor.live_vocals_swap"))
            .open(&mut open)
            .fixed_size(subwindow_size(ctx))
            .show(ctx, |ui| {
                let content_w = ui.max_rect().width();
                ui.set_width(content_w);

                let avail_w = ui.available_width();
                ui.data_mut(|d| {
                    d.insert_temp(egui::Id::new("grid_control_w"), avail_w - LIST_TILE_PAD_H * 2.0 * scale);
                });

                let action_bar_h = 48.0 * scale;
                let scroll_h = (ui.available_height() - action_bar_h - 16.0 * scale).max(40.0);

                // 1. Scroll Area
                egui::ScrollArea::vertical()
                    .id_salt("live_vocals_scroll")
                    .max_height(scroll_h)
                    .show(ui, |ui| {
                        ui.set_width(avail_w);
                        egui::Frame::NONE
                            .inner_margin(egui::Margin::symmetric(
                                (LIST_TILE_PAD_H * scale) as i8,
                                (4.0 * scale) as i8,
                            ))
                            .show(ui, |ui| {
                                // Concert selector
                                let concert_label = t!("config_editor.live_vocals_swap_concert");
                                ui.vertical(|ui| {
                                    ui.add(egui::Label::new(concert_label).wrap());
                                    let avail = ui.available_width();
                                    ui.data_mut(|d| d.insert_temp(egui::Id::new("grid_control_w"), avail));
                                    Gui::run_combo_menu(
                                        ui,
                                        self.id.with("concert"),
                                        &mut self.selected_concert,
                                        &concert_items,
                                        &mut self.search_term,
                                    );
                                    ui.add_space(8.0);
                                });

                                let active: &mut [i32; 6] = if self.selected_concert == 0 {
                                    &mut self.config.live_vocals_swap
                                } else {
                                    self.config.live_vocals_swap_presets.entry(self.selected_concert).or_insert([0; 6])
                                };

                                for i in 0..6 {
                                    let label = t!(
                                        "config_editor.live_vocals_swap_character_n",
                                        index = i + 1
                                    );
                                    ui.vertical(|ui| {
                                        ui.add(egui::Label::new(label).wrap());
                                        let avail = ui.available_width();
                                        ui.data_mut(|d| d.insert_temp(egui::Id::new("grid_control_w"), avail));
                                        Gui::run_combo_menu(
                                            ui,
                                            egui::Id::new("vocals_swap").with(i),
                                            &mut active[i],
                                            &combo_items,
                                            &mut self.search_term,
                                        );
                                        ui.add_space(4.0);
                                    });
                                }
                            });
                        let ime_pad = ime_scroll_padding(ui.ctx());
                        if ime_pad > 0.0 { ui.add_space(ime_pad); }
                    });

                ui.add_space(4.0 * scale);
                ui.separator();
                ui.add_space(4.0 * scale);

                // 2. Action Bar
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                        if ui.add(MaterialButton::outlined(t!("cancel"))).clicked() {
                            open2 = false;
                        }
                        if ui.add(MaterialButton::filled(t!("save"))).clicked() {
                            save_clicked = true;
                            open2 = false;
                        }
                    });
                });
            });

        if self.selected_concert != concert_before {
            LVSW_LAST_CONCERT_ID.store(self.selected_concert, std::sync::atomic::Ordering::Relaxed);
        }

        if save_clicked {
            self.config.live_vocals_swap_presets.retain(|_, v| v.iter().any(|&c| c != 0));
            save_and_reload_config(self.config.clone());
        }

        open &= open2;
        open
    }

    fn window_id(&self) -> Option<egui::Id> {
        Some(self.id)
    }
}
