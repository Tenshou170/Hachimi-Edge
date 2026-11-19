use crate::core::gui::*;

use egui_material3::*;
use rust_i18n::t;


pub struct SimpleMarkdownDialog {
    title: String,
    content: String,
    id: egui::Id,
    cache: egui_commonmark::CommonMarkCache,
    default_height: Option<f32>,
    max_height: Option<f32>,
}

impl SimpleMarkdownDialog {
    pub fn new(title: &str, content: &str) -> SimpleMarkdownDialog {
        SimpleMarkdownDialog {
            title: title.to_owned(),
            content: content.to_owned(),
            id: random_id(),
            cache: egui_commonmark::CommonMarkCache::default(),
            default_height: None,
            max_height: None,
        }
    }

    pub fn new_with_height(title: &str, content: &str, default_height: f32, max_height: f32) -> SimpleMarkdownDialog {
        SimpleMarkdownDialog {
            title: title.to_owned(),
            content: content.to_owned(),
            id: random_id(),
            cache: egui_commonmark::CommonMarkCache::default(),
            default_height: Some(default_height),
            max_height: Some(max_height),
        }
    }
}

impl AppWindow for SimpleMarkdownDialog {
    fn run(&mut self, ctx: &egui::Context) -> bool {
        let mut open = true;
        let mut open2 = true;

        let scale = get_scale(ctx);

        let mut window = new_window(ctx, self.id, &self.title);
        if let Some(h) = self.default_height {
            window = window.default_height(h * scale);
        }
        if let Some(h) = self.max_height {
            window = window.max_height(h * scale);
        }

        window
            .open(&mut open)
            .show(ctx, |ui| {
                egui::TopBottomPanel::bottom(self.id.with("bottom"))
                    .frame(egui::Frame::NONE)
                    .show_inside(ui, |ui| {
                        ui.add_space(4.0 * scale);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                            if ui.add(MaterialButton::filled(t!("ok"))).clicked() {
                                open2 = false;
                            }
                        });
                    });

                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE)
                    .show_inside(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt(self.id.with("scroll"))
                            .show(ui, |ui| {
                                let primary = egui_material3::theme::get_global_color("primary");
                                ui.scope(|ui| {
                                    ui.visuals_mut().override_text_color = None;
                                    ui.visuals_mut().widgets.active.fg_stroke.color = primary;
                                    egui_commonmark::CommonMarkViewer::new()
                                        .show(ui, &mut self.cache, &self.content);
                                });
                            });
                    });
            });

        open && open2
    }
}





pub struct SimpleYesNoDialog {
    title: String,
    content: String,
    callback: Option<Box<dyn FnOnce(bool) + Send + Sync>>,
    id: egui::Id,
}


impl SimpleYesNoDialog {
    pub fn new(
        title: &str,
        content: &str,
        callback: impl FnOnce(bool) + Send + Sync + 'static,
    ) -> SimpleYesNoDialog {
        SimpleYesNoDialog {
            title: title.to_owned(),
            content: content.to_owned(),
            callback: Some(Box::new(callback)),
            id: random_id(),
        }
    }
}


impl AppWindow for SimpleYesNoDialog {
    fn run(&mut self, ctx: &egui::Context) -> bool {
        let mut open = true;
        let mut result = false;
        let mut fired = false;
        let content = self.content.clone();
        let screen_w = ctx.content_rect().width();
        let max_w = (screen_w * 0.80).min(320.0);

        MaterialDialog::new(self.id, &self.title, &mut open)
            .max_width(max_w)
            .min_width(200.0_f32.min(max_w))
            .content(move |ui, _open| {
                ui.label(&content);
            })
            .text_action(t!("no"), || {})
            .filled_action(t!("yes"), || {
                result = true;
                fired = true;
            })
            .show(ctx);

        if fired || !open {
            if let Some(cb) = self.callback.take() {
                cb(result);
            }
            return false;
        }
        open
    }
}


pub struct SimpleOkDialog {
    title: String,
    content: String,
    callback: Option<Box<dyn FnOnce() + Send + Sync>>,
    id: egui::Id,
}


impl SimpleOkDialog {
    pub fn new(
        title: &str,
        content: &str,
        callback: impl FnOnce() + Send + Sync + 'static,
    ) -> SimpleOkDialog {
        SimpleOkDialog {
            title: title.to_owned(),
            content: content.to_owned(),
            callback: Some(Box::new(callback)),
            id: random_id(),
        }
    }
}


impl AppWindow for SimpleOkDialog {
    fn run(&mut self, ctx: &egui::Context) -> bool {
        let mut open = true;
        let mut fired = false;
        let content = self.content.clone();
        let screen_w = ctx.content_rect().width();
        let max_w = (screen_w * 0.80).min(320.0);

        MaterialDialog::new(self.id, &self.title, &mut open)
            .max_width(max_w)
            .min_width(200.0_f32.min(max_w))
            .content(move |ui, _open| {
                ui.label(&content);
            })
            .filled_action(t!("ok"), || {
                fired = true;
            })
            .show(ctx);

        if fired {
            if let Some(cb) = self.callback.take() {
                cb();
            }
            return false;
        }
        open
    }
}



