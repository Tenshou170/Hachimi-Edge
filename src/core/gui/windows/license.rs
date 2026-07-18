use crate::core::gui::*;

use egui_material3::theme::get_global_color;
use rust_i18n::t;



struct FontNotice {
    name: &'static str,
    license: &'static str,
    copyright: &'static str,
}

const FONT_NOTICES: &[FontNotice] = &[
    FontNotice {
        name: "Google Sans Flex",
        license: "SIL Open Font License 1.1",
        copyright: "Copyright (c) 2015 Google LLC. All Rights Reserved.",
    },
    FontNotice {
        name: "Material Symbols Outlined",
        license: "SIL Open Font License 1.1",
        copyright: "Copyright (c) 2026 Google LLC. All Rights Reserved.",
    },
    FontNotice {
        name: "HarmonyOS Sans",
        license: "HarmonyOS Sans Font License Agreement",
        copyright: "Copyright (c) 2021 Huawei Device (Dongguan) Co., Ltd. All rights reserved.",
    },
];

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

                    ui.heading("Font Licenses");
                    ui.label("This software bundles the following fonts:");
                    ui.add_space(4.0);

                    for font in FONT_NOTICES {
                        license_frame.show(ui, |ui| {
                            ui.strong(font.name);
                            ui.label(format!("License: {}", font.license));
                            ui.label(font.copyright);
                        });
                        ui.add_space(4.0);
                    }

                    ui.add_space(4.0);
                    ui.collapsing("SIL OFL 1.1 License Text", |ui| {
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

                    ui.add_space(4.0);
                    ui.collapsing("HarmonyOS Sans License Text", |ui| {
                        license_frame.show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut include_str!(
                                    "../../../../assets/fonts/HarmonyOSSansLicense.txt"
                                ))
                                .font(egui::TextStyle::Monospace)
                                .desired_rows(10)
                                .interactive(false),
                            );
                        });
                    });
                });
            });

        open
    }
}





