use crate::core::gui::*;
use crate::core::hachimi;
use crate::core::Hachimi;
#[allow(unused_imports)]
use egui_material3::theme::get_global_color;
use egui_material3::*;
use rust_i18n::t;
#[allow(unused_imports)]
use std::thread;

pub struct FreeCameraWindow {
    id: egui::Id,
    config: hachimi::Config,
}

impl FreeCameraWindow {
    pub fn new() -> FreeCameraWindow {
        let handle = Hachimi::instance().config.load();
        FreeCameraWindow {
            id: random_id(),
            config: (**handle).clone(),
        }
    }
}

impl AppWindow for FreeCameraWindow {
    fn run(&mut self, ctx: &egui::Context) -> bool {
        let scale = get_scale(ctx);
        let mut open = true;
        let mut open2 = true;
        let mut save_clicked = false;
        let mut reset_clicked = false;

        new_window(ctx, self.id, t!("config_editor.free_camera"))
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

                egui::ScrollArea::vertical()
                    .id_salt("free_camera_scroll")
                    .max_height(scroll_h)
                    .show(ui, |ui| {
                        ui.set_width(avail_w);
                        egui::Frame::NONE
                            .inner_margin(egui::Margin::symmetric(
                                (LIST_TILE_PAD_H * scale) as i8,
                                (4.0 * scale) as i8,
                            ))
                            .show(ui, |#[allow(unused_variables)] ui| {
                                #[cfg(target_os = "windows")]
                                {
                                    use crate::windows::free_camera::FreeCameraMode;

                                    let fc = &mut self.config.windows.free_camera;

                                    section_heading(ui, t!("free_camera.section_general"));
                                    ConfigEditor::list_tile_switch(
                                        ui,
                                        t!("free_camera.enable"),
                                        &mut fc.enabled,
                                        true,
                                    );
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
                                    ConfigEditor::list_tile_combo(
                                        ui,
                                        t!("free_camera.mode"),
                                        "free_camera_mode_win",
                                        &mut fc.mode,
                                        &[
                                            (FreeCameraMode::Free, t!("free_camera.mode_free").as_ref()),
                                            (FreeCameraMode::FirstPerson, t!("free_camera.mode_first_person").as_ref()),
                                            (FreeCameraMode::SelfieStick, t!("free_camera.mode_selfie_stick").as_ref()),
                                        ],
                                    );
                                    ConfigEditor::list_tile_switch(ui, t!("free_camera.show_overlay"), &mut fc.show_overlay, true);
                                    ConfigEditor::list_tile_switch(ui, t!("free_camera.remove_camera_effects"), &mut fc.remove_camera_effects, true);
                                    ConfigEditor::list_tile_switch(ui, t!("free_camera.selfie_use_head_transform"), &mut fc.selfie_use_head_transform, true);
                                    ConfigEditor::list_tile_slider(ui, t!("free_camera.live_fov"), &mut fc.live_fov, 20.0..=160.0, 1.0, 0);
                                    ConfigEditor::list_tile_slider(ui, t!("free_camera.race_fov"), &mut fc.race_fov, 20.0..=160.0, 1.0, 0);
                                    ConfigEditor::list_tile_slider(ui, t!("free_camera.live_move_step"), &mut fc.live_move_step, 0.01..=5.0, 0.01, 2);
                                    ConfigEditor::list_tile_slider(ui, t!("free_camera.race_move_step"), &mut fc.race_move_step, 0.01..=5.0, 0.01, 2);
                                    ConfigEditor::list_tile_slider(ui, t!("free_camera.look_step"), &mut fc.look_step, 0.1..=10.0, 0.1, 1);
                                    ConfigEditor::list_tile_slider(ui, t!("free_camera.mouse_speed"), &mut fc.mouse_speed, 0.5..=50.0, 0.5, 1);
                                    ConfigEditor::list_tile_slider(ui, t!("free_camera.gamepad_deadzone"), &mut fc.gamepad_deadzone, 0.0..=0.5, 0.01, 2);
                                    ConfigEditor::list_tile_slider(ui, t!("free_camera.gamepad_move_speed"), &mut fc.gamepad_move_speed, 0.1..=5.0, 0.1, 1);
                                    ConfigEditor::list_tile_slider(ui, t!("free_camera.gamepad_look_speed"), &mut fc.gamepad_look_speed, 0.1..=5.0, 0.1, 1);

                                    section_heading(ui, t!("free_camera.section_live"));
                                    ConfigEditor::list_tile_switch(ui, t!("free_camera.live_remove_screen_effects"), &mut fc.live_remove_screen_effects, true);
                                    ConfigEditor::list_tile_switch(ui, t!("free_camera.live_disable_character_teleport"), &mut fc.live_disable_character_teleport, true);
                                    ConfigEditor::list_tile_switch(ui, t!("free_camera.live_force_all_characters_visible"), &mut fc.live_force_all_characters_visible, true);

                                    let live_position_choices: Vec<(i32, &str)> = crate::windows::free_camera::LIVE_POSITION_CHOICES
                                        .iter()
                                        .enumerate()
                                        .map(|(i, (name, _))| (i as i32, *name))
                                        .collect();
                                    ConfigEditor::list_tile_combo(ui, t!("free_camera.live_target_position"), "free_camera_live_target_position_win", &mut fc.live_target_position_index, &live_position_choices);

                                    let live_part_choices: Vec<(i32, &str)> = crate::windows::free_camera::LIVE_PART_CHOICES
                                        .iter()
                                        .enumerate()
                                        .map(|(i, (name, _))| (i as i32, *name))
                                        .collect();
                                    ConfigEditor::list_tile_combo(ui, t!("free_camera.live_target_part"), "free_camera_live_target_part_win", &mut fc.live_target_part_index, &live_part_choices);

                                    ConfigEditor::list_tile_number(ui, t!("free_camera.live_selfie_horizontal_stabilization"),
                                        &mut fc.live_selfie_horizontal_stabilization, 0.0..=5.0, 0.01, None);
                                    ConfigEditor::list_tile_number(ui, t!("free_camera.live_selfie_vertical_stabilization"),
                                        &mut fc.live_selfie_vertical_stabilization, 0.0..=5.0, 0.01, None);
                                    ConfigEditor::list_tile_switch(ui, t!("free_camera.live_follow_smooth"), &mut fc.live_follow_smooth, true);
                                    ConfigEditor::list_tile_number(ui, t!("free_camera.live_follow_smooth_pos_step"),
                                        &mut fc.live_follow_smooth_pos_step, 0.02..=1.0, 0.01, None);
                                    ConfigEditor::list_tile_number(ui, t!("free_camera.live_follow_smooth_lookat_step"),
                                        &mut fc.live_follow_smooth_lookat_step, 0.02..=1.0, 0.01, None);

                                    section_heading(ui, t!("free_camera.section_race"));
                                    ConfigEditor::list_tile_number(ui, t!("free_camera.race_target_index"),
                                        &mut fc.race_target_index, -1..=17, 1.0, None);

                                    section_heading(ui, t!("free_camera.section_keybinds"));

                                    let kb = &mut fc.keybinds;
                                    Self::render_keybind_row(ui, t!("free_camera.key_move_forward"), &mut kb.move_forward);
                                    Self::render_keybind_row(ui, t!("free_camera.key_move_back"), &mut kb.move_back);
                                    Self::render_keybind_row(ui, t!("free_camera.key_move_left"), &mut kb.move_left);
                                    Self::render_keybind_row(ui, t!("free_camera.key_move_right"), &mut kb.move_right);
                                    Self::render_keybind_row(ui, t!("free_camera.key_move_down"), &mut kb.move_down);
                                    Self::render_keybind_row(ui, t!("free_camera.key_move_up"), &mut kb.move_up);
                                    Self::render_keybind_row(ui, t!("free_camera.key_look_up"), &mut kb.look_up);
                                    Self::render_keybind_row(ui, t!("free_camera.key_look_down"), &mut kb.look_down);
                                    Self::render_keybind_row(ui, t!("free_camera.key_look_left"), &mut kb.look_left);
                                    Self::render_keybind_row(ui, t!("free_camera.key_look_right"), &mut kb.look_right);
                                    Self::render_keybind_row(ui, t!("free_camera.key_fov_increase"), &mut kb.fov_increase);
                                    Self::render_keybind_row(ui, t!("free_camera.key_fov_decrease"), &mut kb.fov_decrease);
                                    Self::render_keybind_row(ui, t!("free_camera.key_follow_offset_up"), &mut kb.follow_offset_up);
                                    Self::render_keybind_row(ui, t!("free_camera.key_follow_offset_down"), &mut kb.follow_offset_down);
                                    Self::render_keybind_row(ui, t!("free_camera.key_follow_offset_left"), &mut kb.follow_offset_left);
                                    Self::render_keybind_row(ui, t!("free_camera.key_follow_offset_right"), &mut kb.follow_offset_right);
                                    Self::render_keybind_row(ui, t!("free_camera.key_target_previous"), &mut kb.target_previous);
                                    Self::render_keybind_row(ui, t!("free_camera.key_target_next"), &mut kb.target_next);
                                    Self::render_keybind_row(ui, t!("free_camera.key_part_previous"), &mut kb.part_previous);
                                    Self::render_keybind_row(ui, t!("free_camera.key_part_next"), &mut kb.part_next);
                                    Self::render_keybind_row(ui, t!("free_camera.key_reset"), &mut kb.reset);
                                    Self::render_keybind_row(ui, t!("free_camera.key_cycle_mode"), &mut kb.cycle_mode);
                                    Self::render_keybind_row(ui, t!("free_camera.key_reverse"), &mut kb.reverse);
                                }
                            });
                        let ime_pad = ime_scroll_padding(ui.ctx());
                        if ime_pad > 0.0 { ui.add_space(ime_pad); }
                    });

                ui.add_space(4.0 * scale);
                ui.separator();
                ui.add_space(4.0 * scale);

                // Action Bar
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    let error_col = get_global_color("error");
                    if ui.add(MaterialButton::text(t!("config_editor.restore_defaults"))
                        .truncate()
                        .text_color(error_col))
                        .clicked()
                    {
                        reset_clicked = true;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
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

        if reset_clicked {
            #[cfg(target_os = "windows")]
            {
                self.config.windows.free_camera = hachimi::Config::default().windows.free_camera;
            }
        }

        if save_clicked {
            save_and_reload_config(self.config.clone());
        }

        open &= open2;
        open
    }

    fn window_id(&self) -> Option<egui::Id> {
        Some(self.id)
    }
}

impl FreeCameraWindow {
    #[cfg(target_os = "windows")]
    fn render_keybind_row(
        ui: &mut egui::Ui,
        label: impl Into<egui::WidgetText>,
        vk: &mut u16,
    ) {
        let label = label.into();
        let key_label = crate::windows::utils::vk_to_display_label(*vk);
        let secondary_container = get_global_color("secondaryContainer");
        let on_secondary_container = get_global_color("onSecondaryContainer");
        let key_galley = ui.painter().layout_no_wrap(
            key_label.to_string(),
            ui.style().text_styles[&egui::TextStyle::Body].clone(),
            egui::Color32::WHITE,
        );
        let chip_w = key_galley.size().x + 16.0;

        ui.add(egui::Label::new(label).wrap());
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let (chip_rect, _) = ui.allocate_exact_size(egui::vec2(chip_w, 28.0), egui::Sense::hover());
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
                    let ptr = vk as *mut u16 as usize;
                    thread::spawn(move || {
                        let Some(gui_mutex) = Gui::instance() else { return };
                        let mut gui = gui_mutex.lock().unwrap_or_else(|e| e.into_inner());
                        gui.show_window(Box::new(SetKeybindWindow::new(move |result| {
                            if let Some(raw) = result {
                                unsafe {
                                    *(ptr as *mut u16) = raw;
                                }
                            }
                        })));
                    });
                }
            });
        });
        ui.add_space(4.0);
    }
}
