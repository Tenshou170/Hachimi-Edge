use crate::core::gui::*;


use std::sync::Arc;


pub struct PersistentMessageWindow {
    id: egui::Id,
    title: String,
    content: String,
    show: Arc<AtomicBool>,
}


impl PersistentMessageWindow {
    pub fn new(title: &str, content: &str, show: Arc<AtomicBool>) -> PersistentMessageWindow {
        PersistentMessageWindow {
            id: random_id(),
            title: title.to_owned(),
            content: content.to_owned(),
            show,
        }
    }
}


impl AppWindow for PersistentMessageWindow {
    fn run(&mut self, ctx: &egui::Context) -> bool {
        new_window(ctx, self.id, &self.title).show(ctx, |ui| {
            simple_window_layout(
                ui,
                self.id,
                |ui| {
                    ui.centered_and_justified(|ui| {
                        ui.label(&self.content);
                    });
                },
                |_| {},
            );
        });

        self.show.load(atomic::Ordering::Relaxed)
    }
}





