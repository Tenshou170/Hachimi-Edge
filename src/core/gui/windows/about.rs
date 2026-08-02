use crate::core::gui::*;

use egui_material3::{theme::get_global_color, *};
use rust_i18n::t;

use super::*;


pub struct AboutWindow {
    id: egui::Id,
}


impl AboutWindow {
    pub fn new() -> AboutWindow {
        AboutWindow { id: random_id() }
    }
}


impl AppWindow for AboutWindow {
    fn run(&mut self, ctx: &egui::Context) -> bool {
        let scale = get_scale(ctx);
        let mut open = true;

        new_window(ctx, self.id, t!("about.title"))
            .max_width(310.0 * scale)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.with_layout(
                    egui::Layout::top_down(egui::Align::Min).with_cross_justify(true),
                    |ui| {
                        ui.set_min_width(ui.available_width());

                        // Branding row: icon + name/version stacked
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 10.0 * scale;
                            ui.add(Gui::icon_2x(ctx));
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 2.0 * scale;
                                ui.add(egui::Label::new(
                                    egui::RichText::new(t!("hachimi"))
                                        .size(16.0 * scale)
                                        .strong()
                                        .color(get_global_color("onSurface")),
                                ));
                                ui.add(egui::Label::new(
                                    egui::RichText::new(env!("HACHIMI_DISPLAY_VERSION"))
                                        .size(12.0 * scale)
                                        .color(get_global_color("onSurfaceVariant")),
                                ));
                            });
                        });

                        ui.add_space(8.0 * scale);

                        // Copyright — wrap so it never clips on narrow screens
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(
                                    t!("about.copyright", year = Utc::now().year()),
                                )
                                .size(12.0 * scale)
                                .color(get_global_color("onSurfaceVariant")),
                            )
                            .wrap(),
                        );

                        ui.add_space(8.0 * scale);
                        ui.separator();
                        ui.add_space(4.0 * scale);

                        // Action buttons — wrapped so they reflow to a second row on portrait
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing.x = 4.0 * scale;
                            ui.spacing_mut().item_spacing.y = 4.0 * scale;

                            if ui
                                .add(MaterialButton::text(t!("about.view_license")))
                                .clicked()
                            {
                                thread::spawn(|| {
                                    Gui::instance()
                                        .unwrap()
                                        .lock()
                                        .unwrap()
                                        .show_window(Box::new(LicenseWindow::new()));
                                });
                            }

                            if ui
                                .add(MaterialButton::text(t!("about.open_website")))
                                .clicked()
                            {
                                Application::OpenURL(WEBSITE_URL.to_il2cpp_string());
                            }

                            if ui
                                .add(MaterialButton::text(t!("about.view_source_code")))
                                .clicked()
                            {
                                Application::OpenURL(
                                    format!("https://github.com/{}", REPO_PATH)
                                        .to_il2cpp_string(),
                                );
                            }
                        });
                    },
                );
            });

        open
    }
}
