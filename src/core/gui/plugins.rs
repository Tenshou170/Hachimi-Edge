use crate::core::gui::*;


use std::{sync::Arc, os::raw::c_void};


#[derive(Clone)]
pub struct PluginWindow {
    pub id: i32,
    pub title: String,
    pub contents_callback: Option<PluginWindowCallback>,
    pub bottom_callback: Option<PluginWindowCallback>,
    pub userdata: usize,
}


impl AppWindow for PluginWindow {
    fn run(&mut self, ctx: &egui::Context) -> bool {
        let mut open = true;
        let id = egui::Id::new("plugin_AppWindow").with(self.id);

        new_window(ctx, id, &self.title)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);

                simple_window_layout(ui, id,
                    |ui| {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            if let Some(callback) = self.contents_callback {
                                let _ = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
                                    callback(ui as *mut _ as *mut c_void, self.userdata as *mut c_void);
                                })).inspect_err(|_| error!("plugin AppWindow contents callback panicked"));
                            }
                        });
                    },
                    |ui| {
                        if let Some(callback) = self.bottom_callback {
                            let _ = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
                                callback(ui as *mut _ as *mut c_void, self.userdata as *mut c_void);
                            })).inspect_err(|_| error!("plugin AppWindow bottom callback panicked"));
                        }
                    }
                );
            });

        open
    }

    fn plugin_window_id(&self) -> Option<i32> { Some(self.id) }
}


#[derive(Clone)]
pub struct PluginMenuItem {
    pub label: String,
    pub callback: Option<PluginMenuCallback>,
    pub userdata: usize,
}


#[derive(Clone)]
pub struct PluginMenuIcon {
    pub uri: String,
    pub bytes: Arc<[u8]>,
}


#[derive(Clone)]
pub struct PluginMenuSection {
    pub title: Option<String>,
    pub icon: Option<PluginMenuIcon>,
    pub callback: PluginMenuSectionCallback,
    pub userdata: usize,
}


pub fn register_plugin_menu_item(
    label: String,
    callback: Option<PluginMenuCallback>,
    userdata: *mut c_void,
) {
    PLUGIN_MENU_ITEMS.lock().unwrap().push(PluginMenuItem {
        label,
        callback,
        userdata: userdata as usize,
    });
}


pub fn register_plugin_menu_section(callback: PluginMenuSectionCallback, userdata: *mut c_void) {
    PLUGIN_MENU_SECTIONS
        .lock()
        .unwrap()
        .push(PluginMenuSection {
            title: None,
            icon: None,
            callback,
            userdata: userdata as usize,
        });
}


pub fn register_plugin_menu_section_with_icon(
    title: String,
    uri: String,
    bytes: Vec<u8>,
    callback: PluginMenuSectionCallback,
    userdata: *mut c_void,
) -> bool {
    if title.is_empty() || uri.is_empty() || bytes.is_empty() {
        return false;
    }
    PLUGIN_MENU_SECTIONS
        .lock()
        .unwrap()
        .push(PluginMenuSection {
            title: Some(title),
            icon: Some(PluginMenuIcon {
                uri,
                bytes: bytes.into(),
            }),
            callback,
            userdata: userdata as usize,
        });
    true
}


pub fn register_plugin_menu_icon(label: String, uri: String, bytes: Vec<u8>) -> bool {
    if label.is_empty() || uri.is_empty() || bytes.is_empty() {
        return false;
    }
    PLUGIN_MENU_ICONS.lock().unwrap().insert(
        label,
        PluginMenuIcon {
            uri,
            bytes: bytes.into(),
        },
    );
    true
}


pub fn enqueue_plugin_notification(message: String) {
    PLUGIN_NOTIFICATIONS.lock().unwrap().push(message);
}


pub fn show_plugin_AppWindow(
    id: i32,
    title: String,
    contents_callback: Option<PluginWindowCallback>,
    bottom_callback: Option<PluginWindowCallback>,
    userdata: usize,
) {
    let AppWindow = PluginWindow {
        id,
        title,
        contents_callback,
        bottom_callback,
        userdata,
    };

    PLUGIN_WINDOWS_TO_SHOW.lock().unwrap().push(AppWindow);
}


pub fn close_plugin_AppWindow(id: i32) {
    PLUGIN_WINDOWS_TO_CLOSE.lock().unwrap().push(id);
}


pub fn drain_plugin_windows_to_show() -> Vec<PluginWindow> {
    let mut windows = PLUGIN_WINDOWS_TO_SHOW.lock().unwrap();
    std::mem::take(&mut *windows)
}


pub fn take_plugin_windows_to_close() -> Vec<i32> {
    let mut ids = PLUGIN_WINDOWS_TO_CLOSE.lock().unwrap();
    std::mem::take(&mut *ids)
}


pub fn get_plugin_menu_items() -> Vec<PluginMenuItem> {
    PLUGIN_MENU_ITEMS.lock().unwrap().clone()
}


pub fn get_plugin_menu_sections() -> Vec<PluginMenuSection> {
    PLUGIN_MENU_SECTIONS.lock().unwrap().clone()
}


pub fn get_plugin_menu_icon(label: &str) -> Option<PluginMenuIcon> {
    PLUGIN_MENU_ICONS.lock().unwrap().get(label).cloned()
}


pub fn drain_plugin_notifications() -> Vec<String> {
    let mut notifications = PLUGIN_NOTIFICATIONS.lock().unwrap();
    std::mem::take(&mut *notifications)
}








pub fn show_plugin_window(
    id: i32,
    title: String,
    contents_callback: Option<PluginWindowCallback>,
    bottom_callback: Option<PluginWindowCallback>,
    userdata: usize,
) {
    let window = PluginWindow {
        id,
        title,
        contents_callback,
        bottom_callback,
        userdata,
    };

    PLUGIN_WINDOWS_TO_SHOW.lock().unwrap().push(window);
}

pub fn close_plugin_window(id: i32) {
    PLUGIN_WINDOWS_TO_CLOSE.lock().unwrap().push(id);
}

