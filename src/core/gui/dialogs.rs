use crate::core::gui::*;

use egui_material3::*;
use rust_i18n::t;





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
            .content(move |ui| {
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
            .content(move |ui| {
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



