use crate::core::gui::*;
use egui_material3::*;
use rust_i18n::t;

#[derive(PartialEq)]
enum CapState {
    Waiting,
    Captured { raw: RawKeybind, display: String },
}

/// Modal window that listens for the next key press and reports it via a
/// callback. Mirrors the `SetKeybindWindow` pattern from the reference
/// codebase (Hachimi-Edge-Mario0051).
///
/// Usage:
/// ```ignore
/// gui.show_window(Box::new(SetKeybindWindow::new(|result| {
///     let Some(raw) = result else { return };
///     // write raw into config and save
/// })));
/// ```
pub struct SetKeybindWindow {
    id: egui::Id,
    state: CapState,
    callback: Option<Box<dyn FnOnce(Option<RawKeybind>) + Send + Sync>>,
}

impl SetKeybindWindow {
    pub fn new(callback: impl FnOnce(Option<RawKeybind>) + Send + Sync + 'static) -> Self {
        start_keybind_capture();
        Self {
            id: random_id(),
            state: CapState::Waiting,
            callback: Some(Box::new(callback)),
        }
    }

    fn finish(&mut self, result: Option<RawKeybind>) -> bool {
        if let Some(cb) = self.callback.take() {
            cb(result);
        }
        false // close the window
    }
}

impl AppWindow for SetKeybindWindow {
    fn run(&mut self, ctx: &egui::Context) -> bool {
        // Poll for a captured key every frame while waiting.
        if self.state == CapState::Waiting {
            if let Some((raw, display)) = take_keybind_capture() {
                self.state = CapState::Captured { raw, display };
            }
        }

        let mut confirm_raw: Option<RawKeybind> = None;
        let mut cancelled = false;
        let mut rebind = false;
        let mut open = true;

        new_window(ctx, self.id, t!("set_keybind.title"))
            .open(&mut open)
            .show(ctx, |ui| {
                simple_window_layout(
                    ui,
                    self.id,
                    |ui| {
                        ui.centered_and_justified(|ui| match &self.state {
                            CapState::Waiting => {
                                ui.label(t!("set_keybind.press_any_key"));
                            }
                            CapState::Captured { display, .. } => {
                                ui.label(t!(
                                    "set_keybind.bound_key",
                                    key = display.as_str()
                                ));
                            }
                        });
                    },
                    |ui| {
                        if ui.add(MaterialButton::text(t!("cancel"))).clicked() {
                            cancelled = true;
                        }
                        if let CapState::Captured { raw, .. } = &self.state {
                            let raw_copy = *raw;
                            if ui.add(MaterialButton::filled(t!("save"))).clicked() {
                                confirm_raw = Some(raw_copy);
                            }
                            if ui.add(MaterialButton::outlined(t!("retry"))).clicked() {
                                rebind = true;
                            }
                        }
                    },
                );
            });

        if rebind {
            start_keybind_capture();
            self.state = CapState::Waiting;
        }

        if !open || cancelled {
            return self.finish(None);
        }

        if let Some(raw) = confirm_raw {
            return self.finish(Some(raw));
        }

        true
    }
}
