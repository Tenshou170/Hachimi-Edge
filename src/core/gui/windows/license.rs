use crate::core::gui::*;

use egui_material3::theme::get_global_color;
use rust_i18n::t;



pub struct LicenseWindow {
    id: egui::Id,
}


impl LicenseWindow {
    pub fn new() -> LicenseWindow {
        LicenseWindow { id: random_id() }
    }
}


impl AppWindow for LicenseWindow {
    fn run(&mut self, ctx: &egui::Context) -> bool {
        let mut open = true;

        new_window(ctx, self.id, t!("license.title"))
            .open(&mut open)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);

                    // M3 themed frame for license text blocks
                    let license_frame = egui::Frame::NONE
                        .fill(get_global_color("surfaceContainerLow"))
                        .corner_radius(8.0)
                        .inner_margin(egui::Margin::same(8));

                    ui.heading(t!("hachimi"));
                    ui.collapsing(t!("license.gpl_v3_only_notice"), |ui| {
                        license_frame.show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut include_str!("../../../../LICENSE"))
                                    .font(egui::TextStyle::Monospace)
                                    .desired_rows(10)
                                    .interactive(false),
                            );
                        });
                    });
                    ui.separator();

                    ui.heading("Open Font Licenses (OFL)");
                    ui.label(t!("license.ofl_fonts_header"));
                    license_frame.show(ui, |ui| {
                        ui.label(t!("license.font_inter"));
                        ui.label(t!("license.font_font_awesome"));
                    });

                    ui.add_space(4.0);
                    ui.collapsing(t!("license.ofl_notice"), |ui| {
                        license_frame.show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut include_str!(
                                    "../../../../assets/fonts/OFL.txt"
                                ))
                                .font(egui::TextStyle::Monospace)
                                .desired_rows(10)
                                .interactive(false),
                            );
                        });
                    });

                    ui.add_space(10.0);
                    ui.separator();

                    ui.heading(t!("license.font_alibaba_header"));
                    ui.label(t!("license.font_alibaba_body"));
                });
            });

        open
    }
}





