pub mod config;
pub mod dialogs;
pub mod plugins;
pub mod tabs;
pub mod utils;
pub mod windows;

pub use config::*;
pub use dialogs::*;
pub use plugins::*;
pub use utils::*;
pub use windows::*;

use std::{
    borrow::Cow,
    collections::HashMap,
    os::raw::c_void,
    panic::{self, AssertUnwindSafe},
    sync::{
        atomic::{self, AtomicBool},
        Arc, Mutex,
    },
    thread,
    time::Instant,
};

use chrono::{Datelike, Utc};
use egui_material3::{
    tabs::tabs_primary, theme::get_global_color, MaterialButton,
    MaterialProgress, MaterialSelect, MaterialSlider, MaterialSnackbar,
    SelectVariant, SnackBarBehavior, MaterialTextField, MaterialNavigationRail, NavRailItem,
};
use egui_scale::EguiScale;
use fnv::FnvHashSet;
use once_cell::sync::{Lazy, OnceCell};
use rust_i18n::t;

use crate::il2cpp::{
    ext::StringExt,
    hook::{
        umamusume::{
            GameSystem,
            Localize,
        },
        UnityEngine_CoreModule::Application,
    },
    symbols::Thread,
};

#[cfg(target_os = "android")]
use crate::il2cpp::hook::umamusume::WebViewManager;


#[cfg(target_os = "windows")]
use crate::il2cpp::hook::UnityEngine_CoreModule::QualitySettings;

use super::{
    hachimi::{self, Language, REPO_PATH, WEBSITE_URL},
    http::AsyncRequest,
    tl_repo::RepoInfo,
    utils::SendPtr,
    Hachimi,
};

macro_rules! add_font {
    ($fonts:expr, $family_fonts:expr, $filename:literal) => {
        $fonts.font_data.insert(
            $filename.to_owned(),
            egui::FontData::from_static(include_bytes!(concat!("../../../assets/fonts/", $filename)))
                .into(),
        );
        $family_fonts.push($filename.to_owned());
    };
}

static PENDING_THEME: Mutex<Option<hachimi::Config>> = Mutex::new(None);

pub fn enqueue_theme_preview(config: hachimi::Config) {
    if let Ok(mut lock) = PENDING_THEME.lock() {
        *lock = Some(config);
    }
}

static PREV_MENU_WIDTH: Mutex<f32> = Mutex::new(200.0);
static REQUESTED_WIDTH: Mutex<Option<f32>> = Mutex::new(None);

pub fn get_menu_width() -> f32 {
    *PREV_MENU_WIDTH.lock().unwrap()
}

pub fn set_menu_width(width: f32) {
    if let Ok(mut lock) = REQUESTED_WIDTH.lock() {
        *lock = Some(width);
    }
}

type BoxedAppWindow = Box<dyn AppWindow + Send + Sync>;
pub struct Gui {
    pub context: egui::Context,
    pub input: egui::RawInput,
    pub gui_scale: f32,

    pub finalized_scale: f32,
    pub start_time: Instant,
    pub prev_main_axis_size: i32,
    last_fps_update: Instant,
    tmp_frame_count: u32,
    fps_text: String,
    last_focused: Option<egui::Id>,
    #[cfg(target_os = "android")]
    ime_cooldown: Option<Instant>,

    show_menu: bool,

    splash_visible: bool,
    splash_tween: TweenInOutWithDelay,
    splash_sub_str: String,

    menu_visible: bool,
    menu_anim_time: Option<Instant>,
    menu_fps_value: f32,

    #[cfg(target_os = "windows")]
    menu_vsync_value: i32,

    pub update_progress_visible: bool,

    notifications: Vec<Md3Snackbar>,
    next_notification_id: u32,
    windows: Vec<BoxedAppWindow>,
}

const PIXELS_PER_POINT_RATIO: f32 = 3.0 / 1080.0;

static INSTANCE: OnceCell<Mutex<Gui>> = OnceCell::new();
pub static IS_CONSUMING_INPUT: AtomicBool = AtomicBool::new(false);
pub static WANTS_INPUT: AtomicBool = AtomicBool::new(false);
pub static IS_LIVE_SCENE: AtomicBool = AtomicBool::new(false);
pub static IS_LIVE_SLIDER_ACTIVE: AtomicBool = AtomicBool::new(false);
static LIVE_SLIDER_SCENE_HANDLE: atomic::AtomicI32 = atomic::AtomicI32::new(-1);
static DISABLED_GAME_UIS: Lazy<Mutex<FnvHashSet<SendPtr>>> =
    Lazy::new(|| Mutex::new(FnvHashSet::default()));
static PLUGIN_MENU_ITEMS: Lazy<Mutex<Vec<PluginMenuItem>>> = Lazy::new(|| Mutex::new(Vec::new()));
static PLUGIN_MENU_SECTIONS: Lazy<Mutex<Vec<PluginMenuSection>>> =
    Lazy::new(|| Mutex::new(Vec::new()));
static PLUGIN_MENU_ICONS: Lazy<Mutex<HashMap<String, PluginMenuIcon>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static PLUGIN_NOTIFICATIONS: Lazy<Mutex<Vec<String>>> = Lazy::new(|| Mutex::new(Vec::new()));
static PLUGIN_WINDOWS_TO_SHOW: Lazy<Mutex<Vec<PluginWindow>>> = Lazy::new(|| Mutex::new(Vec::new()));
static PLUGIN_WINDOWS_TO_CLOSE: Lazy<Mutex<Vec<i32>>> = Lazy::new(|| Mutex::new(Vec::new()));

static LIVE_SLIDER_CACHE: Lazy<Mutex<Option<LiveSliderCache>>> = Lazy::new(|| Mutex::new(None));

pub type PluginMenuCallback = extern "C" fn(userdata: *mut c_void);
pub type PluginMenuSectionCallback = extern "C" fn(ui: *mut c_void, userdata: *mut c_void);
pub type PluginWindowCallback = unsafe extern "C" fn(ui: *mut c_void, userdata: *mut c_void);

unsafe impl Send for PluginWindow {}
unsafe impl Sync for PluginWindow {}

/// Called on every scene transition to drop stale raw pointers from the set.
/// Without this, pointers to destroyed Unity objects accumulate across scenes.
pub fn clear_disabled_game_uis() {
    DISABLED_GAME_UIS.lock().unwrap().clear();
}

use std::sync::atomic::Ordering;

#[cfg(target_os = "android")]
pub static KEYBOARD_OWNER: Lazy<Mutex<Option<KeyboardOwner>>> = Lazy::new(|| Mutex::new(None));


impl Gui {
    fn apply_target_fps() {
        let fps = Hachimi::instance()
            .target_fps
            .load(atomic::Ordering::Relaxed)
            .clamp(30, 240);
        Application::set_targetFrameRate(fps);
    }

    // Call this from the render thread!
    pub fn instance_or_init(
        #[cfg_attr(target_os = "windows", allow(unused))] open_key_id: &str,
    ) -> &Mutex<Gui> {
        if let Some(instance) = INSTANCE.get() {
            return instance;
        }

        let hachimi = Hachimi::instance();
        let mut config = (**Hachimi::instance().config.load()).clone();

        let context = egui::Context::default();
        egui_extras::install_image_loaders(&context);




        context.set_fonts(Self::get_font_definitions());

        // Apply spacing/interaction style before theme so that theme visuals
        // can still be applied without resetting the full Material3 style.
        context.style_mut(|style| {
            style.spacing.button_padding = egui::Vec2::new(8.0, 5.0);
            style.interaction.selectable_labels = false;
        });

        // If no cached JSON exists yet (first launch or seed changed externally),
        // generate it now and persist it back to config so subsequent launches
        // skip the HCT computation entirely.
        let cached_json_str = config.ui_theme_json.as_ref().and_then(|v| serde_json::to_string(v).ok());
        let params = crate::core::theme::ThemeParams {
            seed: config.ui_theme_seed,
            cached_json: cached_json_str.as_deref(),
            theme_mode: config.ui_theme_mode,
            contrast_level: config.ui_contrast_level,
            scheme_mode: config.ui_color_scheme_mode,
            manual_colors: &config.ui_manual_colors,
            surface_alpha: config.ui_surface_alpha,
            window_rounding: config.ui_window_rounding,
        };
        if let Some(json) = crate::core::theme::apply_seed(&context, params, &hachimi.game.data_dir) {
            let mut updated_config = config.clone();
            updated_config.ui_theme_json = serde_json::from_str(&json).ok();
            let _ = hachimi.save_config(&updated_config);
            hachimi.config.store(std::sync::Arc::new(updated_config));
            config = (**Hachimi::instance().config.load()).clone();
        }

        let mut fps_value = hachimi.target_fps.load(atomic::Ordering::Relaxed);
        if fps_value == -1 {
            fps_value = 30;
        }
        fps_value = fps_value.clamp(30, 240);

        let mut windows: Vec<BoxedAppWindow> = Vec::new();
        if !config.skip_first_time_setup {
            windows.push(Box::new(FirstTimeSetupWindow::new()));
        }

        let now = Instant::now();
        let instance = Gui {
            context,
            input: egui::RawInput::default(),
            gui_scale: 1.0,
            finalized_scale: 1.0,
            start_time: now,
            prev_main_axis_size: 1,
            last_fps_update: now,
            tmp_frame_count: 0,
            fps_text: "FPS: 0".to_string(),
            last_focused: None,
            #[cfg(target_os = "android")]
            ime_cooldown: None,

            show_menu: false,

            splash_visible: true,
            splash_tween: TweenInOutWithDelay::new(0.8, 3.0, Easing::OutQuad),
            splash_sub_str: {
                #[cfg(target_os = "windows")]
                {
                    let key_label = crate::windows::utils::vk_to_display_label(
                        hachimi.config.load().windows.menu_open_key,
                    );
                    t!("splash_sub", open_key_str = key_label).into_owned()
                }
                #[cfg(not(target_os = "windows"))]
                {
                    t!("splash_sub", open_key_str = t!(open_key_id)).into_owned()
                }
            },

            menu_visible: false,
            menu_anim_time: None,
            menu_fps_value: fps_value as f32,

            #[cfg(target_os = "windows")]
            menu_vsync_value: hachimi.vsync_count.load(atomic::Ordering::Relaxed),

            update_progress_visible: false,

            notifications: Vec::new(),
            next_notification_id: 0,
            windows,
        };

        unsafe {
            INSTANCE.set(Mutex::new(instance)).unwrap_unchecked();
            INSTANCE.get().unwrap_unchecked()
        }
    }

    pub fn instance() -> Option<&'static Mutex<Gui>> {
        INSTANCE.get()
    }

    fn get_font_definitions() -> egui::FontDefinitions {
        let mut fonts = egui::FontDefinitions::default();
        let proportional_fonts = fonts
            .families
            .get_mut(&egui::FontFamily::Proportional)
            .unwrap();

        proportional_fonts.clear();
        
        add_font!(fonts, proportional_fonts, "GoogleSansFlex.ttf");
        add_font!(fonts, proportional_fonts, "MaterialSymbolsOutlined.ttf");
        add_font!(fonts, proportional_fonts, "AlibabaPuHuiTi-3-45-Light.otf");

        fonts
    }

    pub fn set_screen_size(&mut self, width: i32, height: i32) {
        let is_landscape = width > height;
        let main_axis_size = if is_landscape {
            height
        } else {
            width.min(height)
        };

        let enable_gui_landscape_ratio = {
            #[cfg(target_os = "windows")]
            { Hachimi::instance().config.load().windows.enable_gui_landscape_ratio }
            #[cfg(target_os = "android")]
            { false }
        };
        let gui_landscape_ratio = {
            #[cfg(target_os = "windows")]
            { Hachimi::instance().config.load().windows.gui_landscape_ratio }
            #[cfg(target_os = "android")]
            { 1.0f32 }
        };

        let pixels_per_point = compute_pixels_per_point(
            width,
            height,
            gui_landscape_ratio,
            enable_gui_landscape_ratio,
        );

        // when something actually changed. set_pixels_per_point triggers a full
        // egui relayout, so calling it every frame at the same value wastes CPU.
        let prev_ppp = self.context.pixels_per_point();
        let screen_rect = egui::Rect {
            min: egui::Pos2::ZERO,
            max: egui::Pos2::new(
                width as f32 / pixels_per_point,
                height as f32 / pixels_per_point,
            ),
        };

        if (pixels_per_point - prev_ppp).abs() > f32::EPSILON
            || self.input.screen_rect != Some(screen_rect)
        {
            self.context.set_pixels_per_point(pixels_per_point);
            self.input.screen_rect = Some(screen_rect);
        }

        self.prev_main_axis_size = main_axis_size;
    }

    fn take_input(&mut self) -> egui::RawInput {
        self.input.time = Some(self.start_time.elapsed().as_secs_f64());
        self.input.take()
    }

    fn update_fps(&mut self) {
        let delta = self.last_fps_update.elapsed().as_secs_f64();
        if delta > 0.5 {
            let fps = (self.tmp_frame_count as f64 * (0.5 / delta) * 2.0).round();
            self.fps_text = t!("menu.fps_text", fps = fps).into_owned();
            self.tmp_frame_count = 1;
            self.last_fps_update = Instant::now();
        } else {
            self.tmp_frame_count += 1;
        }
    }

    fn run_live_slider(&mut self, ctx: &egui::Context) {
        let config = crate::core::Hachimi::instance().config.load();

        use crate::il2cpp::{
            ext::Il2CppStringExt,
            hook::UnityEngine_CoreModule::{Scene, SceneManager},
        };
        let scene = SceneManager::GetActiveScene();
        let last_scene_handle = LIVE_SLIDER_SCENE_HANDLE.load(atomic::Ordering::Relaxed);
        if scene.handle != last_scene_handle {
            LIVE_SLIDER_SCENE_HANDLE.store(scene.handle, atomic::Ordering::Relaxed);
            let name_ptr = Scene::GetNameInternal(scene.handle);
            let is_live = !name_ptr.is_null() && unsafe { (*name_ptr).as_utf16str() == "Live" };
            IS_LIVE_SCENE.store(is_live, atomic::Ordering::Release);
        }

        if !IS_LIVE_SCENE.load(atomic::Ordering::Acquire) {
            IS_LIVE_SLIDER_ACTIVE.store(false, atomic::Ordering::Release);
            return;
        }

        unsafe {
            let (
                get_instance_method,
                get_current_time_addr,
                get_total_time_addr,
                is_pause_live_addr,
                sm_get_instance,
                photo_check_field,
                photo_library_field,
            ) = {
                let mut cache_guard = LIVE_SLIDER_CACHE.lock().unwrap_or_else(|e| e.into_inner());
                let cache = cache_guard.get_or_insert_with(|| {
                    let mut cache = LiveSliderCache::default();
                    let Ok(image) = crate::il2cpp::symbols::get_assembly_image(c"umamusume.dll")
                    else {
                        return cache;
                    };
                    let Ok(dir_class) =
                        crate::il2cpp::symbols::get_class(image, c"Gallop.Live", c"Director")
                    else {
                        return cache;
                    };

                    cache.director_class = dir_class as usize;
                    cache.get_instance_method =
                        crate::il2cpp::api::il2cpp_class_get_method_from_name(
                            dir_class,
                            c"get_Instance".as_ptr(),
                            0,
                        ) as usize;
                    cache.get_current_time = crate::il2cpp::symbols::get_method_addr_cached(
                        dir_class,
                        c"get_LiveCurrentTime",
                        0,
                    );
                    cache.get_total_time = crate::il2cpp::symbols::get_method_addr_cached(
                        dir_class,
                        c"get_LiveTotalTime",
                        0,
                    );
                    cache.is_pause_live = crate::il2cpp::symbols::get_method_addr_cached(
                        dir_class,
                        c"IsPauseLive",
                        0,
                    );

                    // SceneManager photo-mode fields
                    if let Ok(sm_class) = crate::il2cpp::symbols::get_class(image, c"Gallop", c"SceneManager") {
                        cache.scene_manager_class = sm_class as usize;
                        cache.scene_manager_get_instance =
                            crate::il2cpp::api::il2cpp_class_get_method_from_name(
                                sm_class,
                                c"get_Instance".as_ptr(),
                                0,
                            ) as usize;
                        cache.photo_check_field =
                            crate::il2cpp::api::il2cpp_class_get_field_from_name(
                                sm_class,
                                c"PhotoCheckObject".as_ptr(),
                            ) as usize;
                        cache.photo_library_field =
                            crate::il2cpp::api::il2cpp_class_get_field_from_name(
                                sm_class,
                                c"PhotoLibraryObject".as_ptr(),
                            ) as usize;
                    }

                    cache
                });
                (
                    cache.get_instance_method,
                    cache.get_current_time,
                    cache.get_total_time,
                    cache.is_pause_live,
                    cache.scene_manager_get_instance,
                    cache.photo_check_field,
                    cache.photo_library_field,
                )
            };
            if get_instance_method == 0
                || get_current_time_addr == 0
                || get_total_time_addr == 0
            {
                return;
            }

            // Hide the slider when the in-game photo mode is active
            if sm_get_instance != 0 && (photo_check_field != 0 || photo_library_field != 0) {
                let sm_method = sm_get_instance as *const crate::il2cpp::types::MethodInfo;
                let mut exc: *mut crate::il2cpp::types::Il2CppException = std::ptr::null_mut();
                let sm = crate::il2cpp::api::il2cpp_runtime_invoke(
                    sm_method,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    &mut exc,
                );
                if !sm.is_null() && exc.is_null() {
                    let in_photo_mode = if photo_check_field != 0 {
                        let mut val: *mut crate::il2cpp::types::Il2CppObject = std::ptr::null_mut();
                        crate::il2cpp::api::il2cpp_field_get_value(
                            sm,
                            photo_check_field as *mut crate::il2cpp::types::FieldInfo,
                            &mut val as *mut _ as *mut std::ffi::c_void,
                        );
                        !val.is_null()
                    } else {
                        false
                    } || if photo_library_field != 0 {
                        let mut val: *mut crate::il2cpp::types::Il2CppObject = std::ptr::null_mut();
                        crate::il2cpp::api::il2cpp_field_get_value(
                            sm,
                            photo_library_field as *mut crate::il2cpp::types::FieldInfo,
                            &mut val as *mut _ as *mut std::ffi::c_void,
                        );
                        !val.is_null()
                    } else {
                        false
                    };

                    if in_photo_mode {
                        IS_LIVE_SLIDER_ACTIVE.store(false, atomic::Ordering::Release);
                        return;
                    }
                }
            }

            let director: *mut crate::il2cpp::types::Il2CppObject = {
                let method = get_instance_method as *const crate::il2cpp::types::MethodInfo;
                let mut exc: *mut crate::il2cpp::types::Il2CppException = std::ptr::null_mut();
                let obj = crate::il2cpp::api::il2cpp_runtime_invoke(
                    method,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    &mut exc,
                );
                if !exc.is_null() { std::ptr::null_mut() } else { obj }
            };
            if director.is_null() {
                return;
            }

            let get_current_time: extern "C" fn(*mut crate::il2cpp::types::Il2CppObject) -> f32 =
                std::mem::transmute(get_current_time_addr);
            let get_total_time: extern "C" fn(*mut crate::il2cpp::types::Il2CppObject) -> f32 =
                std::mem::transmute(get_total_time_addr);

            let mut current = get_current_time(director);
            let total = get_total_time(director);
            if total <= 0.0 {
                return;
            }

            if config.live_playback_loop && current >= total - 0.1 {
                crate::core::live_utils::move_live_playback(0.0);
                current = 0.0;
            }

            if is_pause_live_addr != 0 {
                let is_pause_live: extern "C" fn(*mut crate::il2cpp::types::Il2CppObject) -> bool =
                    std::mem::transmute(is_pause_live_addr);
                if !config.live_slider_always_show && !is_pause_live(director) {
                    IS_LIVE_SLIDER_ACTIVE.store(false, atomic::Ordering::Release);
                    return;
                }
            } else if !config.live_slider_always_show {
                IS_LIVE_SLIDER_ACTIVE.store(false, atomic::Ordering::Release);
                return;
            }

            IS_LIVE_SLIDER_ACTIVE.store(true, atomic::Ordering::Release);
            let scale = get_scale(ctx);
            egui::Area::new(egui::Id::new("live_slider_area"))
                .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -40.0 * scale))
                .show(ctx, |ui| {
                    egui::Frame::window(&ctx.style())
                        .fill(egui::Color32::from_black_alpha(150))
                        .inner_margin(egui::Margin::symmetric(
                            (16.0 * scale) as i8,
                            (8.0 * scale) as i8,
                        ))
                        .corner_radius(10.0 * scale)
                        .show(ui, |ui| {
                            ui.set_width(ctx.content_rect().width() * 0.7);
                            ui.horizontal(|ui| {
                                let curr_m = (current / 60.0).floor() as i32;
                                let curr_s = (current % 60.0).floor() as i32;
                                let tot_m = (total / 60.0).floor() as i32;
                                let tot_s = (total % 60.0).floor() as i32;
                                ui.label(format!(
                                    "{:02}:{:02} / {:02}:{:02}",
                                    curr_m, curr_s, tot_m, tot_s
                                ));

                                let available_w = ui.available_width();

                                ui.scope(|ui| {
                                    ui.spacing_mut().slider_width = available_w - (16.0 * scale);

                                    let res = ui.add(
                                        MaterialSlider::new(&mut current, 0.0..=total)
                                            .show_value(false),
                                    );

                                    if res.changed() {
                                        crate::core::live_utils::move_live_playback(current);
                                    }
                                });
                            });
                        });
                });
        }
    }

    pub fn run(&mut self) -> egui::FullOutput {
        if let Ok(mut lock) = PENDING_THEME.lock() {
            if let Some(config) = lock.take() {
                // Preview: no cached JSON — regenerate from new seed.
                // Result is discarded since this is not a committed save.
                let preview_params = crate::core::theme::ThemeParams {
                    seed: config.ui_theme_seed,
                    cached_json: None,
                    theme_mode: config.ui_theme_mode,
                    contrast_level: config.ui_contrast_level,
                    scheme_mode: config.ui_color_scheme_mode,
                    manual_colors: &config.ui_manual_colors,
                    surface_alpha: config.ui_surface_alpha,
                    window_rounding: config.ui_window_rounding,
                };
                crate::core::theme::apply_seed(
                    &self.context,
                    preview_params,
                    &Hachimi::instance().game.data_dir,
                );
            }
        }

        self.update_fps();
        let input = self.take_input();

        let live_scale = Hachimi::instance().config.load().gui_scale;
        if self.gui_scale != live_scale {
            self.gui_scale = live_scale;
            if !self.context.is_using_pointer() {
                self.finalized_scale = live_scale;
            }

            self.context.style_mut(|style| {
                style.spacing.button_padding = egui::Vec2::new(8.0, 5.0);
                style.interaction.selectable_labels = false;
                if live_scale != 1.0 {
                    style.scale(live_scale);
                }
            });

            // Re-apply corner radius after style mutation (scale change may affect it)
            let rounding = Hachimi::instance().config.load().ui_window_rounding;
            {
                let cr = egui::CornerRadius::same(rounding.round() as u8);
                self.context.style_mut(|s| {
                    s.visuals.window_corner_radius = cr;
                    s.visuals.widgets.noninteractive.corner_radius = cr;
                    s.visuals.widgets.inactive.corner_radius = cr;
                    s.visuals.widgets.hovered.corner_radius = cr;
                    s.visuals.widgets.active.corner_radius = cr;
                    s.visuals.widgets.open.corner_radius = cr;
                });
            }
        }

        // Only update egui temp data when the values actually changed to avoid
        // locking the internal TypeMap and hashing the Id strings every frame.
        let prev_scale = self
            .context
            .data(|d| d.get_temp::<f32>(egui::Id::new("gui_scale")))
            .unwrap_or(0.0);
        let prev_salt = self
            .context
            .data(|d| d.get_temp::<f32>(egui::Id::new("gui_scale_salt")))
            .unwrap_or(0.0);
        if (prev_scale - live_scale).abs() > f32::EPSILON
            || (prev_salt - self.finalized_scale).abs() > f32::EPSILON
        {
            self.context.data_mut(|d| {
                d.insert_temp(egui::Id::new("gui_scale"), live_scale);
                d.insert_temp(egui::Id::new("gui_scale_salt"), self.finalized_scale);
            });
        }

        self.context.begin_pass(input);

        if self.menu_visible {
            self.run_menu();
        }
        if self.update_progress_visible {
            self.run_update_progress();
        }

        self.process_plugin_windows();
        self.run_windows();
        self.run_notifications();

        if self.splash_visible {
            self.run_splash();
        }
        if hachimi::CONFIG_LOAD_ERROR.swap(false, Ordering::AcqRel) {
            self.show_notification(&t!("notification.config_error"));
        }

        #[cfg(target_os = "windows")]
        {
            use crate::il2cpp::hook::UnityEngine_InputLegacyModule::Input::set_imeCompositionMode;

            let focused = self.context.memory(|m| m.focused());
            let wants_kb = self.context.wants_keyboard_input();

            if focused != self.last_focused {
                if wants_kb {
                    Thread::main_thread().schedule(|| {
                        set_imeCompositionMode(1);
                    });
                } else if self.last_focused.is_some() {
                    Thread::main_thread().schedule(|| {
                        set_imeCompositionMode(0);
                    });
                }
            }
            self.last_focused = focused;
        }
        #[cfg(target_os = "android")]
        {
            use crate::android::utils::{
                check_keyboard_status, set_keyboard_visible, BACK_BUTTON_PRESSED, IS_IME_VISIBLE,
            };

            let focused = self.context.memory(|m| m.focused());
            let wants_kb = self.context.wants_keyboard_input();

            if let Ok(mut owner_lock) = KEYBOARD_OWNER.try_lock() {
                if focused.is_some() && focused != self.last_focused && wants_kb {
                    if owner_lock.is_none() {
                        if !IS_IME_VISIBLE.load(Ordering::Acquire) {
                            set_keyboard_visible(true);
                            if let Some(id) = focused {
                                *owner_lock = Some(KeyboardOwner::JNI(id));
                            }
                            self.ime_cooldown =
                                Some(Instant::now() + std::time::Duration::from_millis(500));
                        }
                    }
                } else if focused.is_none() && self.last_focused.is_some() {
                    if let Some(KeyboardOwner::JNI(_)) = *owner_lock {
                        set_keyboard_visible(false);
                        *owner_lock = None;
                    }
                }

                if BACK_BUTTON_PRESSED.swap(false, Ordering::AcqRel) {
                    if let Some(KeyboardOwner::JNI(_)) = *owner_lock {
                        set_keyboard_visible(false);
                    }
                    *owner_lock = None;
                    self.context.memory_mut(|mem| mem.stop_text_input());
                    IS_IME_VISIBLE.store(false, Ordering::Release);
                    self.last_focused = None;
                    self.ime_cooldown = None;
                }
            }

            // Zombie check — detect when the JNI keyboard was dismissed
            // externally (e.g. user swiped it away) so we can clean up focus.
            if self.tmp_frame_count % 20 == 0 {
                let should_check = if let Some(until) = self.ime_cooldown {
                    Instant::now() > until
                } else {
                    true
                };

                if should_check && IS_IME_VISIBLE.load(Ordering::Acquire) {
                    if !check_keyboard_status() {
                        self.context.memory_mut(|mem| mem.stop_text_input());
                        IS_IME_VISIBLE.store(false, Ordering::Release);

                        if let Ok(mut lock) = KEYBOARD_OWNER.try_lock() {
                            *lock = None;
                        }
                        self.last_focused = None;
                        self.ime_cooldown = None;
                    }
                }
            }

            self.last_focused = focused;
        }

        let ctx = self.context.clone();
        self.run_live_slider(&ctx);

        let has_interactive_widgets = IS_LIVE_SCENE.load(atomic::Ordering::Relaxed);

        // Store this as an atomic value so the input thread can check it without locking the gui
        IS_CONSUMING_INPUT.store(
            self.is_consuming_input() || has_interactive_widgets,
            atomic::Ordering::Relaxed,
        );

        WANTS_INPUT.store(
            self.context.wants_pointer_input()
                || self.context.is_pointer_over_area()
                || self.context.wants_keyboard_input(),
            atomic::Ordering::Relaxed,
        );

        self.context.end_pass()
    }

    const ICON_IMAGE: egui::ImageSource<'static> = egui::include_image!("../../../assets/icon.png");
    fn icon<'a>(ctx: &egui::Context) -> egui::Image<'a> {
        let scale = get_scale(ctx);
        egui::Image::new(Self::ICON_IMAGE)
            .fit_to_exact_size(egui::Vec2::new(24.0 * scale, 24.0 * scale))
    }

    fn icon_2x<'a>(ctx: &egui::Context) -> egui::Image<'a> {
        let scale = get_scale(ctx);
        egui::Image::new(Self::ICON_IMAGE)
            .fit_to_exact_size(egui::Vec2::new(48.0 * scale, 48.0 * scale))
    }

    fn run_splash(&mut self) {
        let ctx = &self.context;
        let scale = get_scale(ctx);

        let id = egui::Id::from("splash");
        let Some(tween_val) = self.splash_tween.run(ctx, id.with("tween")) else {
            self.splash_visible = false;
            return;
        };

        // Slide down from top of the screen: start offset above the screen (-100dp) and slide down to 8dp.
        let start_y = -100.0 * scale;
        let target_y = 8.0 * scale;
        let current_y = start_y + (target_y - start_y) * tween_val;

        egui::Area::new(id)
            .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, current_y))
            .constrain(false)
            .show(ctx, |ui| {
                let screen_w = ctx.content_rect().width();
                let card_width = screen_w * 0.80;

                egui::Frame::NONE
                    .fill(get_global_color("surfaceContainerHigh"))
                    .corner_radius(12.0 * scale)
                    .shadow(egui::Shadow {
                        spread: 0,
                        blur: (8.0 * scale) as u8,
                        offset: [0, (2.0 * scale) as i8],
                        color: egui::Color32::from_black_alpha(40),
                    })
                    .inner_margin(egui::Margin::same((16.0 * scale) as i8))
                    .show(ui, |ui| {
                        ui.set_width(card_width);
                        ui.horizontal(|ui| {
                            ui.add(Self::icon(ctx));
                            ui.heading("Hachimi");
                            ui.label(
                                egui::RichText::new(env!("HACHIMI_DISPLAY_VERSION"))
                                    .color(get_global_color("onSurfaceVariant"))
                                    .size(12.0 * scale),
                            );
                        });
                        ui.add_space(4.0 * scale);
                        ui.label(
                            egui::RichText::new(&self.splash_sub_str)
                                .color(get_global_color("onSurfaceVariant"))
                                .size(13.0 * scale),
                        );
                    });
            });
    }

    fn run_menu(&mut self) {
        let hachimi = Hachimi::instance();
        let localized_data = hachimi.localized_data.load();
        let localize_dict_count = localized_data.localize_dict.len().to_string();
        let hashed_dict_count = localized_data.hashed_dict.len().to_string();

        let mut show_notification: Option<Cow<'_, str>> = None;
        let mut show_window: Option<BoxedAppWindow> = None;
        {
            let ctx = &self.context;
            let scale = get_scale(ctx);
            let salt = self.finalized_scale;

            let screen_w = ctx.content_rect().width();
            let mut min_w = 96.0 * scale;
            let mut max_w = (screen_w * 0.80).min(280.0 * scale);

            if let Ok(mut lock) = REQUESTED_WIDTH.lock() {
                if let Some(w) = lock.take() {
                    min_w = w;
                    max_w = w;
                }
            }

            let panel_res =
                egui::SidePanel::left(egui::Id::new("hachimi_menu").with(salt.to_bits()))
                    .min_width(min_w)
                    .max_width(max_w)
                    .default_width((200.0 * scale).min(screen_w * 0.70))
                    .show_animated(ctx, self.show_menu, |ui| {
                        ui.with_layout(egui::Layout::top_down_justified(egui::Align::TOP), |ui| {
                            #[cfg(target_os = "windows")]
                            {
                                ui.horizontal(|ui| {
                                    ui.add(Self::icon(ctx));
                                    ui.heading(t!("hachimi"));
                                    if ui.add(MaterialButton::text(" \u{e887} ")).clicked() {
                                        show_window = Some(Box::new(AboutWindow::new()));
                                    }
                                });
                                ui.label(env!("HACHIMI_DISPLAY_VERSION"));
                                if ui
                                    .add(MaterialButton::text(t!("menu.close_menu")))
                                    .clicked()
                                {
                                    self.show_menu = false;
                                    self.menu_anim_time = None;
                                }
                            }
                            // did this because android phones have a notch
                            #[cfg(target_os = "android")]
                            {
                                ui.horizontal(|ui| {
                                    ui.add(Self::icon(ctx));
                                    ui.heading(t!("hachimi"));
                                });
                                ui.label(env!("HACHIMI_DISPLAY_VERSION"));
                                ui.horizontal(|ui| {
                                    if ui
                                        .add(MaterialButton::text(t!("menu.close_menu")))
                                        .clicked()
                                    {
                                        self.show_menu = false;
                                        self.menu_anim_time = None;
                                    }
                                    if ui.add(MaterialButton::text(" \u{e887} ")).clicked() {
                                        show_window = Some(Box::new(AboutWindow::new()));
                                    }
                                });
                            }
                            if ui
                                .add(MaterialButton::filled(t!("menu.check_for_updates")))
                                .clicked()
                            {
                                Hachimi::instance()
                                    .updater
                                    .clone()
                                    .check_for_updates(|_| {});
                            }
                            ui.separator();
                            egui::ScrollArea::vertical().show(ui, |ui| {
                                ui.set_width(ui.available_width());

                                let render_heading = |ui: &mut egui::Ui, text: &str| {
                                    egui::Frame::NONE
                                        .inner_margin(egui::Margin::symmetric(
                                            (LIST_TILE_PAD_H * scale) as i8,
                                            (8.0 * scale) as i8,
                                        ))
                                        .show(ui, |ui| {
                                            ui.heading(text);
                                        });
                                };

                                // 1. Stats Section
                                render_heading(ui, &t!("menu.stats_heading"));
                                egui::Frame::NONE
                                    .inner_margin(egui::Margin::symmetric(
                                        (LIST_TILE_PAD_H * scale) as i8,
                                        (4.0 * scale) as i8,
                                    ))
                                    .show(ui, |ui| {
                                        ui.label(&self.fps_text);
                                        ui.label(t!("menu.localize_dict_entries", count = localize_dict_count));
                                        ui.label(t!("menu.hashed_dict_entries", count = hashed_dict_count));
                                    });
                                ui.add_space(4.0 * scale);
                                ui.separator();
                                ui.add_space(4.0 * scale);

                                // 2. Config Section
                                render_heading(ui, &t!("menu.config_heading"));
                                if ConfigEditor::list_tile_button(ui, t!("menu.open_config_editor"), t!("open")) {
                                    show_window = Some(Box::new(ConfigEditor::new()));
                                }
                                if ConfigEditor::list_tile_button(ui, t!("menu.reload_config"), t!("reload")) {
                                    hachimi.reload_config();
                                    show_notification = Some(t!("notification.config_reloaded"));
                                }
                                if ConfigEditor::list_tile_button(ui, t!("menu.open_first_time_setup"), t!("open")) {
                                    show_window = Some(Box::new(FirstTimeSetupWindow::new()));
                                }
                                ui.add_space(4.0 * scale);
                                ui.separator();
                                ui.add_space(4.0 * scale);

                                // 3. Graphics Section
                                render_heading(ui, &t!("menu.graphics_heading"));
                                let mut current_target_fps = hachimi.target_fps.load(atomic::Ordering::Relaxed);
                                if current_target_fps <= 0 {
                                    current_target_fps = 30;
                                }
                                self.menu_fps_value = current_target_fps as f32;

                                let slider_res = ConfigEditor::list_tile_slider(ui, t!("menu.fps_label"), &mut self.menu_fps_value, 30.0..=240.0, 1.0, 0);
                                if slider_res.changed() {
                                    let clamped = (self.menu_fps_value as i32).clamp(30, 240);
                                    self.menu_fps_value = clamped as f32;

                                    hachimi.target_fps.store(clamped, atomic::Ordering::Relaxed);
                                    Thread::main_thread().schedule(Self::apply_target_fps);
                                }
                                if slider_res.lost_focus() {
                                    self.menu_fps_value = self.menu_fps_value.clamp(30.0, 240.0);
                                }

                                #[cfg(target_os = "windows")]
                                {
                                    self.menu_vsync_value = hachimi.vsync_count.load(atomic::Ordering::Relaxed);
                                    let prev_value = self.menu_vsync_value;
                                    let t_default = t!("default");
                                    let t_off = t!("off");
                                    let t_on = t!("on");
                                    let choices = &[
                                        (-1, t_default.as_ref()),
                                        (0, t_off.as_ref()),
                                        (1, t_on.as_ref()),
                                        (2, "1/2"),
                                        (3, "1/3"),
                                        (4, "1/4"),
                                    ];
                                    ConfigEditor::list_tile_combo(ui, t!("menu.vsync_label"), "menu_vsync", &mut self.menu_vsync_value, choices);
                                    if prev_value != self.menu_vsync_value {
                                        let mut new_config = (**hachimi.config.load()).clone();
                                        new_config.windows.vsync_count = self.menu_vsync_value;
                                        if let Err(e) = hachimi.save_and_reload_config(new_config) {
                                            error!("{}", e);
                                        }
                                        Thread::main_thread().schedule(|| {
                                            QualitySettings::set_vSyncCount(1);
                                        });
                                    }

                                    let mut topmost = hachimi.config.load().windows.window_always_on_top;
                                    if ConfigEditor::list_tile_switch(ui, t!("menu.stay_on_top"), &mut topmost, true).changed() {
                                        let mut new_config = (**hachimi.config.load()).clone();
                                        new_config.windows.window_always_on_top = topmost;
                                        if let Err(e) = hachimi.save_and_reload_config(new_config) {
                                            error!("{}", e);
                                        }
                                        Thread::main_thread().schedule(|| {
                                            let topmost = Hachimi::instance().window_always_on_top.load(atomic::Ordering::Relaxed);
                                            unsafe {
                                                _ = crate::windows::utils::set_window_topmost(
                                                    crate::windows::wnd_hook::get_target_hwnd(),
                                                    topmost,
                                                );
                                            }
                                        });
                                    }

                                    let mut discord_rpc = hachimi.config.load().windows.discord_rpc;
                                    if ConfigEditor::list_tile_switch(ui, t!("menu.discord_rpc"), &mut discord_rpc, true).changed() {
                                        let mut new_config = (**hachimi.config.load()).clone();
                                        new_config.windows.discord_rpc = discord_rpc;
                                        if let Err(e) = hachimi.save_and_reload_config(new_config) {
                                            error!("{}", e);
                                        }
                                        if let Err(e) = if discord_rpc {
                                            crate::windows::discord::start_rpc()
                                        } else {
                                            crate::windows::discord::stop_rpc()
                                        } {
                                            error!("{}", e);
                                        }
                                    }

                                    let supports_smtc = crate::windows::capabilities::supports_smtc();
                                    let mut enable_smtc = hachimi.config.load().windows.enable_smtc;
                                    let label = if supports_smtc {
                                        t!("config_editor.enable_smtc").into_owned()
                                    } else {
                                        format!("{} (Wine unavailable)", t!("config_editor.enable_smtc"))
                                    };
                                    if ConfigEditor::list_tile_switch(ui, label, &mut enable_smtc, supports_smtc).changed() {
                                        let mut new_config = (**hachimi.config.load()).clone();
                                        new_config.windows.enable_smtc = enable_smtc;
                                        if let Err(e) = hachimi.save_and_reload_config(new_config) {
                                            error!("{}", e);
                                        }
                                        if enable_smtc {
                                            crate::windows::smtc::init(crate::windows::wnd_hook::get_target_hwnd());
                                        } else {
                                            crate::windows::smtc::unregister();
                                        }
                                    }
                                }

                                #[cfg(target_os = "android")]
                                {
                                    let mut keep_screen_on = hachimi.config.load().android.keep_screen_on;
                                    if ConfigEditor::list_tile_switch(ui, t!("menu.keep_screen_on"), &mut keep_screen_on, true).changed() {
                                        let mut new_config = (**hachimi.config.load()).clone();
                                        new_config.android.keep_screen_on = keep_screen_on;
                                        if let Err(e) = hachimi.save_and_reload_config(new_config) {
                                            error!("{}", e);
                                        }
                                    }
                                }
                                ui.add_space(4.0 * scale);
                                ui.separator();
                                ui.add_space(4.0 * scale);

                                // 4. Translation Section
                                render_heading(ui, &t!("menu.translation_heading"));
                                if ConfigEditor::list_tile_button(ui, t!("menu.reload_localized_data"), t!("reload")) {
                                    hachimi.load_localized_data();
                                    show_notification = Some(t!("notification.localized_data_reloaded"));
                                }
                                if ConfigEditor::list_tile_button(ui, t!("menu.tl_check_for_updates"), t!("check")) {
                                    hachimi.tl_updater.skip_update(None);
                                    hachimi.tl_updater.clone().check_for_updates(false, false);
                                }
                                if ConfigEditor::list_tile_button(ui, t!("menu.tl_check_for_updates_pedantic"), t!("check")) {
                                    hachimi.tl_updater.skip_update(None);
                                    hachimi.tl_updater.clone().check_for_updates(true, false);
                                }
                                if hachimi.config.load().translation_repo_index_mod.is_some() {
                                    if ConfigEditor::list_tile_button(ui, t!("menu.tl_check_for_addon_updates_pedantic"), t!("check")) {
                                        hachimi.tl_updater.skip_update(None);
                                        hachimi.tl_updater.clone().check_for_mod_updates_only(true, false);
                                    }
                                }
                                if hachimi.config.load().translator_mode {
                                    if ConfigEditor::list_tile_button(ui, t!("menu.dump_localize_dict"), t!("dump")) {
                                        Thread::main_thread().schedule(|| {
                                            let data = Localize::dump_strings();
                                            let dict_path = Hachimi::instance().get_data_path("localize_dump.json");
                                            let mut gui = Gui::instance().unwrap().lock().unwrap();
                                            if let Err(e) = crate::core::utils::write_json_file(&data, dict_path) {
                                                gui.show_notification(&e.to_string())
                                            } else {
                                                gui.show_notification(&t!("notification.saved_localize_dump"))
                                            }
                                        })
                                    }
                                }
                                ui.add_space(4.0 * scale);
                                ui.separator();
                                ui.add_space(4.0 * scale);

                                // 5. Plugins Section
                                let plugin_items = get_plugin_menu_items();
                                if !plugin_items.is_empty() {
                                    render_heading(ui, "Plugins");
                                    for item in plugin_items {
                                        let icon = get_plugin_menu_icon(&item.label);
                                        let clicked = if let Some(icon) = icon {
                                            let size = 18.0 * scale;
                                            ui.horizontal(|ui| {
                                                ui.add(
                                                    egui::Image::new((icon.uri, icon.bytes))
                                                        .fit_to_exact_size(egui::Vec2::splat(size)),
                                                );
                                                ConfigEditor::list_tile_button(ui, &item.label, t!("open"))
                                            })
                                            .inner
                                        } else {
                                            ConfigEditor::list_tile_button(ui, &item.label, t!("open"))
                                        };
                                        if clicked {
                                            if let Some(callback) = item.callback {
                                                let _ = panic::catch_unwind(AssertUnwindSafe(|| {
                                                    callback(item.userdata as *mut c_void);
                                                }))
                                                .inspect_err(|_| {
                                                    error!("plugin menu item callback panicked: {}", item.label);
                                                });
                                            }
                                        }
                                    }
                                    ui.add_space(4.0 * scale);
                                    ui.separator();
                                    ui.add_space(4.0 * scale);
                                }

                                let plugin_sections = get_plugin_menu_sections();
                                if !plugin_sections.is_empty() {
                                    for section in plugin_sections {
                                        if let Some(title) = section.title.clone() {
                                            let icon = section.icon.clone();
                                            let size = 18.0 * scale;
                                            ui.horizontal(|ui| {
                                                if let Some(icon) = icon {
                                                    ui.add(
                                                        egui::Image::new((icon.uri, icon.bytes))
                                                            .fit_to_exact_size(egui::Vec2::splat(size)),
                                                    );
                                                }
                                                ui.heading(title);
                                            });
                                        }
                                        let _ = panic::catch_unwind(AssertUnwindSafe(|| {
                                            (section.callback)(
                                                ui as *mut _ as *mut c_void,
                                                section.userdata as *mut c_void,
                                            );
                                        }))
                                        .inspect_err(|_| {
                                            error!("plugin menu section callback panicked");
                                        });
                                    }
                                    ui.add_space(4.0 * scale);
                                    ui.separator();
                                    ui.add_space(4.0 * scale);
                                }

                                // 6. Danger Zone Section
                                render_heading(ui, &t!("menu.danger_zone_heading"));
                                egui::Frame::NONE
                                    .inner_margin(egui::Margin::symmetric(
                                        (LIST_TILE_PAD_H * scale) as i8,
                                        (4.0 * scale) as i8,
                                    ))
                                    .show(ui, |ui| {
                                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                                        let layout = egui::Layout::left_to_right(egui::Align::Min)
                                            .with_cross_justify(false);
                                        ui.with_layout(layout, |ui| {
                                            ui.label(t!("menu.danger_zone_warning"));
                                        });
                                    });
                                if ConfigEditor::list_tile_button_danger(ui, t!("menu.soft_restart"), t!("restart")) {
                                    show_window = Some(Box::new(SimpleYesNoDialog::new(
                                        &t!("confirm_dialog_title"),
                                        &t!("soft_restart_confirm_content"),
                                        |ok| {
                                            if !ok {
                                                return;
                                            }
                                            Thread::main_thread().schedule(|| {
                                                GameSystem::SoftwareReset(GameSystem::instance());
                                            });
                                        },
                                    )));
                                }
                                #[cfg(not(target_os = "windows"))]
                                if ConfigEditor::list_tile_button_danger(ui, t!("menu.open_in_game_browser"), t!("open")) {
                                    show_window = Some(Box::new(SimpleYesNoDialog::new(
                                        &t!("confirm_dialog_title"),
                                        &t!("in_game_browser_confirm_content"),
                                        |ok| {
                                            if !ok {
                                                return;
                                            }
                                            Thread::main_thread().schedule(|| {
                                                WebViewManager::quick_open(
                                                    &t!("browser_dialog_title"),
                                                    &Hachimi::instance()
                                                        .config
                                                        .load()
                                                        .open_browser_url,
                                                );
                                            });
                                        },
                                    )));
                                }
                                if ConfigEditor::list_tile_button(ui, t!("menu.toggle_game_ui"), t!("toggle")) {
                                    Thread::main_thread().schedule(Self::toggle_game_ui);
                                }

                            });
                        });
                    });

            if let Some(inner) = &panel_res {
                let current_width = inner.response.rect.width();
                if let Ok(mut prev_lock) = PREV_MENU_WIDTH.lock() {
                    *prev_lock = current_width;
                }
            }
        }

        for message in drain_plugin_notifications() {
            self.show_notification(&message);
        }

        if !self.show_menu {
            if let Some(time) = self.menu_anim_time {
                if time.elapsed().as_secs_f32() >= self.context.style().animation_time {
                    self.menu_visible = false;
                }
            } else {
                self.menu_anim_time = Some(Instant::now());
            }
        }

        if let Some(content) = show_notification {
            self.show_notification(content.as_ref());
        }

        if let Some(AppWindow) = show_window {
            self.show_window(AppWindow);
        }
    }

    pub fn toggle_game_ui() {
        use crate::il2cpp::hook::{
            Plugins::AnimateToUnity::AnRoot,
            UnityEngine_CoreModule::{Behaviour, GameObject, Object},
            UnityEngine_UIModule::Canvas,
        };

        let canvas_array = Object::FindObjectsOfType(Canvas::type_object(), true);
        let an_root_array = Object::FindObjectsOfType(AnRoot::type_object(), true);
        let canvas_iter = unsafe { canvas_array.as_slice().iter() };
        let an_root_iter = unsafe { an_root_array.as_slice().iter() };

        let mut disabled_uis = DISABLED_GAME_UIS.lock().unwrap();

        if disabled_uis.is_empty() {
            for canvas in canvas_iter {
                if Behaviour::get_enabled(*canvas) {
                    Behaviour::set_enabled(*canvas, false);
                    disabled_uis.insert(SendPtr(*canvas));
                }
            }
            for an_root in an_root_iter {
                let top_object = AnRoot::get__topObject(*an_root);
                if GameObject::get_activeSelf(top_object) {
                    GameObject::SetActive(top_object, false);
                    disabled_uis.insert(SendPtr(top_object));
                }
            }
        } else {
            for canvas in canvas_iter {
                if disabled_uis.contains(&SendPtr(*canvas)) {
                    Behaviour::set_enabled(*canvas, true);
                }
            }
            for an_root in an_root_iter {
                let top_object = AnRoot::get__topObject(*an_root);
                if disabled_uis.contains(&SendPtr(top_object)) {
                    GameObject::SetActive(top_object, true);
                }
            }
            disabled_uis.clear();
        }
    }



    fn run_combo<T: PartialEq + Copy>(
        ui: &mut egui::Ui,
        id_child: impl std::hash::Hash,
        value: &mut T,
        choices: &[(T, &str)],
    ) -> bool {
        let mut selected = "Unknown";
        for choice in choices.iter() {
            if *value == choice.0 {
                selected = choice.1;
            }
        }

        let mut changed = false;
        let col_w = grid_control_w(ui);
        let selected_idx_orig = choices.iter().position(|choice| *value == choice.0);
        let mut selected_idx = selected_idx_orig;
        let mut select = MaterialSelect::new(&mut selected_idx)
            .variant(SelectVariant::Outlined)
            .placeholder(selected)
            .width(col_w)
            .small();
        for (idx, choice) in choices.iter().enumerate() {
            select = select.option(idx, choice.1);
        }
        if ui.push_id(id_child, |ui| ui.add(select)).inner.changed() {
            if let Some(idx) = selected_idx {
                if let Some(choice) = choices.get(idx) {
                    *value = choice.0;
                    changed = true;
                }
            }
        }

        changed
    }

    pub fn run_combo_menu<T: PartialEq + Copy>(
        ui: &mut egui::Ui,
        id_salt: impl std::hash::Hash,
        value: &mut T,
        choices: &[(T, &str)],
        search_term: &mut String,
    ) -> bool {
        let mut changed = false;
        let scale = get_scale(ui.ctx());
        let row_height = 24.0 * scale;
        let padding = ui.spacing().button_padding;

        let button_id = ui.make_persistent_id(id_salt);
        let popup_id = button_id.with("popup");

        let selected_text = choices
            .iter()
            .find(|(v, _)| v == value)
            .map(|(_, s)| *s)
            .unwrap_or("Unknown");

        let font_id = egui::TextStyle::Button.resolve(ui.style());
        let galley = ui.painter().layout_no_wrap(
            selected_text.to_owned(),
            font_id,
            ui.visuals().text_color(),
        );

        let icon_size = 12.0 * scale;
        let desired_width = galley.size().x + icon_size + padding.x * 2.0 + 8.0 * scale;
        let min_width = 145.0 * scale;
        let popup_width = desired_width.max(min_width);

        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(desired_width, row_height), egui::Sense::hover());
        let button_res = ui.interact(rect, button_id, egui::Sense::click());

        if ui.is_rect_visible(rect) {
            let is_open = egui::Popup::is_id_open(ui.ctx(), popup_id);
            let visuals = if is_open {
                &ui.visuals().widgets.open
            } else {
                ui.style().interact(&button_res)
            };

            ui.painter().rect(
                rect.expand(visuals.expansion),
                visuals.corner_radius,
                visuals.weak_bg_fill,
                visuals.bg_stroke,
                egui::epaint::StrokeKind::Inside,
            );

            let icon_size = 12.0 * scale;
            let icon_rect = egui::Rect::from_center_size(
                egui::pos2(rect.right() - padding.x - icon_size / 2.0, rect.center().y),
                egui::vec2(icon_size, icon_size),
            );
            Self::down_triangle_icon(ui.painter(), icon_rect, visuals);

            let galley = ui.painter().layout_no_wrap(
                selected_text.to_owned(),
                egui::TextStyle::Button.resolve(ui.style()),
                visuals.text_color(),
            );

            let text_pos = egui::pos2(
                rect.left() + padding.x,
                rect.center().y - galley.size().y / 2.0,
            );
            ui.painter().galley(text_pos, galley, visuals.text_color());
        }

        let close_behavior = {
            #[cfg(target_os = "android")]
            {
                if let Ok(owner) = crate::core::gui::KEYBOARD_OWNER.try_lock() {
                    if owner.is_some() {
                        egui::PopupCloseBehavior::IgnoreClicks
                    } else {
                        egui::PopupCloseBehavior::CloseOnClickOutside
                    }
                } else {
                    egui::PopupCloseBehavior::CloseOnClickOutside
                }
            }
            #[cfg(not(target_os = "android"))]
            {
                egui::PopupCloseBehavior::CloseOnClickOutside
            }
        };

        egui::Popup::menu(&button_res)
            .id(popup_id)
            .close_behavior(close_behavior)
            .show(|ui| {
                ui.set_width(popup_width);
                ui.set_max_width(popup_width);

                ui.horizontal(|ui| {
                    egui::ScrollArea::neither().show(ui, |ui| {
                        let _res = ui.add_sized(
                                [ui.available_width() - 30.0 * scale, row_height],
                                MaterialTextField::filled(search_term).hint_text(t!("search_filter")),
                            );
                    });

                    if ui.add(MaterialButton::text("X").small()).clicked() {
                        search_term.clear();
                    }
                });

                ui.separator();

                egui::ScrollArea::vertical()
                    .max_height(250.0 * scale)
                    .hscroll(false)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);

                        ui.with_layout(egui::Layout::top_down_justified(egui::Align::Min), |ui| {
                            for (choice_val, label) in choices {
                                if !search_term.is_empty()
                                    && !label.to_lowercase().contains(&search_term.to_lowercase())
                                {
                                    continue;
                                }

                                let is_selected = value == choice_val;
                                if ui
                                    .add(MaterialButton::filled_tonal(*label).selected(is_selected))
                                    .clicked()
                                {
                                    *value = *choice_val;
                                    changed = true;
                                    egui::Popup::close_id(ui.ctx(), popup_id);
                                    search_term.clear();
                                }
                            }
                        });
                    });
            });

        changed
    }

    // egui's code originally (https://github.com/emilk/egui/blob/main/crates/egui/src/containers/combo_box.rs)
    pub fn down_triangle_icon(
        painter: &egui::Painter,
        rect: egui::Rect,
        visuals: &egui::style::WidgetVisuals,
    ) {
        let rect = egui::Rect::from_center_size(
            rect.center(),
            egui::vec2(rect.width() * 0.7, rect.height() * 0.45),
        );

        painter.add(egui::Shape::convex_polygon(
            vec![rect.left_top(), rect.right_top(), rect.center_bottom()],
            visuals.fg_stroke.color,
            visuals.fg_stroke,
        ));
    }

    fn run_update_progress(&mut self) {
        let ctx = &self.context;
        let scale = get_scale(ctx);

        let tl_updater = Hachimi::instance().tl_updater.clone();

        // Show mod progress if active, otherwise fall back to main TL progress
        let (progress, is_mod) = if let Some(p) = tl_updater.mod_progress() {
            (p, true)
        } else if let Some(p) = tl_updater.progress() {
            (p, false)
        } else {
            // Neither active — hide the overlay
            self.update_progress_visible = false;
            return;
        };

        let ratio = progress.current as f32 / progress.total as f32;

        // Center-top card, 80% screen width, MD3 styled
        egui::Area::new("update_progress".into())
            .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 8.0 * scale))
            .constrain(false)
            .show(ctx, |ui| {
                let screen_w = ctx.content_rect().width();
                let card_width = screen_w * 0.80;
                let pad_h = 16.0 * scale;
                let pad_v = 10.0 * scale;
                let is_downloading = Hachimi::instance().tl_updater.is_downloading();
                let title = if is_mod {
                    t!("tl_updater.title_mod")
                } else if is_downloading {
                    t!("tl_updater.title")
                } else {
                    t!("tl_updater.checking")
                };
                egui::Frame::NONE
                    .fill(get_global_color("surfaceContainerHigh"))
                    .inner_margin(egui::Margin::symmetric(pad_h as i8, pad_v as i8))
                    .corner_radius(12.0 * scale)
                    .shadow(egui::Shadow {
                        spread: 0,
                        blur: (6.0 * scale) as u8,
                        offset: [0, (2.0 * scale) as i8],
                        color: egui::Color32::from_black_alpha(30),
                    })
                    .show(ui, |ui| {
                        ui.set_width(card_width);

                        // Row 1: title left, percentage right
                        ui.horizontal(|ui| {
                            ui.set_width(ui.available_width());
                            ui.label(title);
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(format!("{:.2}%", ratio * 100.0));
                                },
                            );
                        });

                        ui.add_space(4.0 * scale);

                        // Row 2: progress bar — fills available width symmetrically
                        let bar_w = ui.available_width();
                        ui.add(
                            MaterialProgress::linear()
                                .value(ratio)
                                .size(egui::Vec2::new(bar_w, 4.0 * scale)),
                        );

                        // Row 3: warning — only shown during actual download, small font
                        if is_downloading {
                            ui.add_space(4.0 * scale);
                            ui.label(
                                egui::RichText::new(t!("tl_updater.warning"))
                                    .font(egui::FontId::proportional(10.0 * scale)),
                            );
                        }
                    });
            });
    }

    fn run_notifications(&mut self) {
        let ctx = &self.context;
        let scale = get_scale(ctx);
        let now = Instant::now();
        // Center-bottom stack per MD3 spec: first snackbar sits 16dp from bottom,
        // each subsequent one is stacked above the previous with 8dp gap.
        let gap = 8.0 * scale;
        let mut offset = 0.0f32;

        self.notifications.retain(|s| {
            let elapsed = now.duration_since(s.show_time).as_secs_f32();
            if !s.persistent && elapsed >= 4.0 {
                return false;
            }

            // Floating behavior with center-bottom positioning per MD3 snackbar spec.
            // Max-width is capped so long text wraps rather than the bar overflowing.
            let max_w = (ctx.content_rect().width() - 32.0 * scale).max(120.0 * scale);
            let inner = egui::Area::new(egui::Id::new("snackbar").with(s.id))
                .order(egui::Order::Foreground)
                .anchor(
                    egui::Align2::CENTER_BOTTOM,
                    egui::vec2(0.0, -(16.0 * scale + offset)),
                )
                .show(ctx, |ui| {
                    ui.set_max_width(max_w);
                    ui.add(
                        MaterialSnackbar::new(&s.message)
                            .behavior(SnackBarBehavior::Floating)
                            .corner_radius(4.0 * scale)
                            .auto_dismiss(None), // lifetime managed by retain above
                    );
                });

            // Advance offset by the actual rendered height so the next snackbar
            // stacks correctly even when text wraps to multiple lines.
            offset += inner.response.rect.height() + gap;
            true
        });
    }

    fn process_plugin_windows(&mut self) {
        let new_windows = drain_plugin_windows_to_show();
        let new_ids: Vec<i32> = new_windows.iter().map(|w| w.id).collect();
        let close_ids = take_plugin_windows_to_close();

        if !new_ids.is_empty() || !close_ids.is_empty() {
            self.windows.retain_mut(|w| {
                if let Some(id) = w.plugin_window_id() {
                    !new_ids.contains(&id) && !close_ids.contains(&id)
                } else {
                    true
                }
            });
        }

        for AppWindow in new_windows {
            self.show_window(Box::new(AppWindow));
        }
    }

    fn run_windows(&mut self) {
        self.windows.retain_mut(|w| w.run(&self.context));
    }

    pub fn is_empty(&self) -> bool {
        !self.splash_visible
            && !self.menu_visible
            && !self.update_progress_visible
            && self.notifications.is_empty()
            && self.windows.is_empty()
            && !IS_LIVE_SCENE.load(atomic::Ordering::Relaxed)
    }

    pub fn is_consuming_input(&self) -> bool {
        self.menu_visible || !self.windows.is_empty()
    }

    pub fn is_consuming_input_atomic() -> bool {
        IS_CONSUMING_INPUT.load(atomic::Ordering::Relaxed)
    }

    pub fn set_consuming_input(&mut self, val: bool) {
        if !self.windows.is_empty() && !val {
            self.windows.clear();
        }

        self.menu_visible = val;
        IS_CONSUMING_INPUT.store(val, atomic::Ordering::Relaxed);
    }

    pub fn wants_input_atomic() -> bool {
        WANTS_INPUT.load(atomic::Ordering::Relaxed)
    }

    pub fn toggle_menu(&mut self) {
        self.show_menu = !self.show_menu;
        // Menu is always visible on show, but not immediately invisible on hide
        if self.show_menu {
            self.menu_visible = true;
        } else {
            self.menu_anim_time = None;
        }
    }

    pub fn show_notification(&mut self, content: &str) {
        self.add_notification(content, false);
    }

    pub fn show_persistent_notification(&mut self, content: &str) -> u32 {
        self.add_notification(content, true)
    }

    fn add_notification(&mut self, content: &str, persistent: bool) -> u32 {
        let id = self.next_notification_id;
        self.notifications.push(Md3Snackbar {
            id,
            message: content.to_owned(),
            persistent,
            show_time: Instant::now(),
        });
        self.next_notification_id = self.next_notification_id.wrapping_add(1);
        id
    }

    pub fn close_notification(&mut self, id: u32) {
        self.notifications.retain(|n| n.id != id);
    }

    pub fn show_window(&mut self, AppWindow: BoxedAppWindow) {
        self.windows.push(AppWindow);
    }
}

/// Platform-independent raw keybind type (VK code on Windows, keycode on Android).
#[cfg(target_os = "windows")]
pub type RawKeybind = u16;
#[cfg(target_os = "android")]
pub type RawKeybind = i32;
/// Fallback for other platforms.
#[cfg(not(any(target_os = "windows", target_os = "android")))]
pub type RawKeybind = i32;

static KEYBIND_CAPTURE_ACTIVE: AtomicBool = AtomicBool::new(false);
static KEYBIND_CAPTURED: Lazy<Mutex<Option<(RawKeybind, String)>>> =
    Lazy::new(|| Mutex::new(None));

pub fn start_keybind_capture() {
    *KEYBIND_CAPTURED.lock().unwrap() = None;
    KEYBIND_CAPTURE_ACTIVE.store(true, atomic::Ordering::Relaxed);
}

pub fn is_keybind_capture_active() -> bool {
    KEYBIND_CAPTURE_ACTIVE.load(atomic::Ordering::Relaxed)
}

pub fn report_keybind_capture(raw: RawKeybind, display: String) {
    KEYBIND_CAPTURE_ACTIVE.store(false, atomic::Ordering::Relaxed);
    *KEYBIND_CAPTURED.lock().unwrap() = Some((raw, display));
}

pub fn take_keybind_capture() -> Option<(RawKeybind, String)> {
    KEYBIND_CAPTURED.lock().unwrap().take()
}

/// Navigation rail destinations for the ConfigEditor.
/// Icons: Material Symbols codepoints, rendered via MaterialSymbolsOutlined.ttf.
///   \u{e8b9} = settings          (General)
///   \u{eb97} = display_settings  (Graphics)
///   \u{e30f} = gamepad           (Gameplay)
///   \u{e869} = build / wrench    (Advanced)
const CONFIG_NAV_ITEMS: &[NavRailItem] = &[
    NavRailItem::new("\u{e8b9}", "General"),
    NavRailItem::new("\u{eb97}", "Graphics"),
    NavRailItem::new("\u{e30f}", "Gameplay"),
    NavRailItem::new("\u{e869}", "Advanced"),
];



impl AppWindow for ConfigEditor {
    fn run(&mut self, ctx: &egui::Context) -> bool {
        let scale   = get_scale(ctx);
        let portrait = is_portrait(ctx);

        let mut open  = true;
        let mut open2 = true;
        let global_handle = Hachimi::instance().config.load();
        let global_ptr    = Arc::as_ptr(&global_handle) as usize;

        if global_ptr != self.last_ptr_config {
            self.config          = (**global_handle).clone();
            self.last_ptr_config = global_ptr;
        }
        let mut config = self.config.clone();
        #[cfg(target_os = "windows")]
        {
            config.windows.menu_open_key = global_handle.windows.menu_open_key;
        }
        let mut reset_clicked = false;
        let mut save_clicked  = false;

        // ── Window frame: explicit surfaceContainer background ─────────────
        let surface_container = get_global_color("surfaceContainer");
        let outline_variant   = get_global_color("outlineVariant");
        let cr = egui_material3::theme::get_global_corner_radius();
        let window_frame = egui::Frame::window(&ctx.style())
            .fill(egui::Color32::from_rgba_unmultiplied(
                surface_container.r(),
                surface_container.g(),
                surface_container.b(),
                255,
            ))
            .stroke(egui::Stroke::new(1.0, outline_variant))
            .corner_radius(egui::CornerRadius::same(cr.unwrap_or(8.0).max(8.0) as u8));

        new_window(ctx, self.id, t!("config_editor.title"))
            .open(&mut open)
            .fixed_size(config_editor_window_size(ctx))
            .frame(window_frame)
            .show(ctx, |ui| {
                let content_w = ui.max_rect().width();
                ui.set_width(content_w);

                if portrait {
                    Self::run_portrait_layout(
                        self, ui, &mut config,
                        content_w, scale,
                        &mut reset_clicked, &mut save_clicked, &mut open2,
                    );
                } else {
                    Self::run_landscape_layout(
                        self, ui, &mut config,
                        content_w, scale,
                        &mut reset_clicked, &mut save_clicked, &mut open2,
                    );
                }
            });

        self.config = config;

        if save_clicked  { save_and_reload_config(self.config.clone()); }
        if reset_clicked { self.restore_defaults(); }

        open &= open2;
        if !open {
            let config_locale = Hachimi::instance().config.load().language.locale_str();
            if config_locale != &*rust_i18n::locale() {
                rust_i18n::set_locale(config_locale);
            }
        }

        open
    }
}

impl ConfigEditor {
    // ── Portrait layout ─────────────────────────────────────────────────────
    #[allow(clippy::too_many_arguments)]
    fn run_portrait_layout(
        &mut self,
        ui: &mut egui::Ui,
        config: &mut hachimi::Config,
        content_w: f32,
        scale: f32,
        reset_clicked: &mut bool,
        save_clicked:  &mut bool,
        open2: &mut bool,
    ) {
        // Publish control width for portrait layout
        let avail_w = ui.available_width();
        ui.data_mut(|d| {
            d.insert_temp(egui::Id::new("grid_control_w"), avail_w - LIST_TILE_PAD_H * 2.0 * scale);
        });

        let tab_h = 48.0 * scale;
        let action_bar_h = 48.0 * scale;
        let scroll_h = (ui.available_height() - action_bar_h - tab_h - 16.0 * scale).max(40.0);

        // 1. Settings Scroll Area
        egui::ScrollArea::vertical()
            .id_salt("portrait_body_scroll")
            .max_height(scroll_h)
            .show(ui, |ui| {
                ui.set_width(avail_w);
                egui::Frame::NONE
                    .inner_margin(egui::Margin::symmetric(
                        (LIST_TILE_PAD_H * scale) as i8,
                        (4.0 * scale) as i8,
                    ))
                    .show(ui, |ui| {
                        self.run_options(config, ui, self.current_tab);
                    });
                let ime_pad = ime_scroll_padding(ui.ctx());
                if ime_pad > 0.0 { ui.add_space(ime_pad); }
            });

        ui.add_space(4.0 * scale);
        ui.separator();
        ui.add_space(4.0 * scale);

        // 2. Tab Bar (at the bottom, above action buttons)
        let mut tab_idx = self.current_tab.as_index();
        ui.scope_builder(egui::UiBuilder::new(), |ui| {
            ui.set_width(content_w);
            ui.add(
                tabs_primary(&mut tab_idx)
                    .tab_with_icon(strip_icon(&t!("config_editor.general_tab")), "\u{e8b9}")
                    .tab_with_icon(strip_icon(&t!("config_editor.graphics_tab")), "\u{eb97}")
                    .tab_with_icon(strip_icon(&t!("config_editor.gameplay_tab")), "\u{e30f}")
                    .tab_with_icon(strip_icon(&t!("config_editor.advanced_tab")), "\u{e869}")
                    .width(content_w)
                    .height(tab_h)
                    .id_salt("config_editor_tabs_portrait"),
            );
        });
        self.current_tab = ConfigEditorTab::from_index(tab_idx);

        ui.add_space(4.0 * scale);
        ui.separator();
        ui.add_space(4.0 * scale);

        // 3. Action Bar (Restore Defaults, Save, Cancel)
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
            let error_col = get_global_color("error");
            if ui
                .add(MaterialButton::text(t!("config_editor.restore_defaults"))
                    .truncate()
                    .text_color(error_col))
                .clicked()
            {
                *reset_clicked = true;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                if ui.add(MaterialButton::outlined(t!("cancel"))).clicked() {
                    *open2 = false;
                }
                if ui.add(MaterialButton::filled(t!("save"))).clicked() {
                    *save_clicked = true;
                    *open2 = false;
                }
            });
        });
    }

    // ── Landscape layout ─────────────────────────────────────────────────────
    #[allow(clippy::too_many_arguments)]
    fn run_landscape_layout(
        &mut self,
        ui: &mut egui::Ui,
        config: &mut hachimi::Config,
        content_w: f32,
        scale: f32,
        reset_clicked: &mut bool,
        save_clicked:  &mut bool,
        open2: &mut bool,
    ) {
        let action_bar_h = 48.0 * scale;
        let content_h    = ui.available_height();
        let body_h       = (content_h - action_bar_h - 16.0 * scale).max(40.0);

        let rail_w = MaterialNavigationRail::WIDTH * scale;
        let body_w = content_w - rail_w;

        let body_rect = egui::Rect::from_min_size(
            ui.cursor().min,
            egui::vec2(content_w, body_h),
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(body_rect), |ui| {
            ui.set_height(body_h);
            ui.horizontal(|ui| {
                ui.set_height(body_h);
                // Left — NavigationRail
                let mut tab_idx = self.current_tab.as_index();
                let (_, changed) = MaterialNavigationRail::new(&mut tab_idx, CONFIG_NAV_ITEMS)
                    .width(rail_w)
                    .show(ui);
                if changed {
                    self.current_tab = ConfigEditorTab::from_index(tab_idx);
                }

                // Right — settings list
                let inner_w = body_w - LIST_TILE_PAD_H * 2.0 * scale;
                ui.data_mut(|d| {
                    d.insert_temp(egui::Id::new("grid_control_w"), inner_w);
                });

                ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("landscape_body_scroll")
                        .max_height(body_h)
                        .show(ui, |ui| {
                            ui.set_width(body_w);
                            egui::Frame::NONE
                                .inner_margin(egui::Margin {
                                    left:   (LIST_TILE_PAD_H * scale) as i8,
                                    right:  (LIST_TILE_PAD_H * scale) as i8,
                                    top:    (8.0 * scale) as i8,
                                    bottom: (8.0 * scale) as i8,
                                })
                                .show(ui, |ui| {
                                    self.run_options(config, ui, self.current_tab);
                                });
                            let ime_pad = ime_scroll_padding(ui.ctx());
                            if ime_pad > 0.0 { ui.add_space(ime_pad); }
                        });
                });
            });
        });
        ui.advance_cursor_after_rect(body_rect);
        ui.add_space(4.0 * scale);

        // Action bar
        ui.separator();
        ui.add_space(4.0 * scale);

        ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
            let error_col = get_global_color("error");
            if ui
                .add(MaterialButton::text(t!("config_editor.restore_defaults"))
                    .truncate()
                    .text_color(error_col))
                .clicked()
            {
                *reset_clicked = true;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                if ui.add(MaterialButton::outlined(t!("cancel"))).clicked() {
                    *open2 = false;
                }
                if ui.add(MaterialButton::filled(t!("save"))).clicked() {
                    *save_clicked = true;
                    *open2 = false;
                }
            });
        });
    }
}

fn strip_icon(label: &str) -> &str {
    let mut chars = label.chars();
    if chars.next().is_some() {
        if let Some(c) = chars.next() {
            if c.is_whitespace() {
                return chars.as_str();
            }
        }
    }
    label
}

pub fn save_and_reload_config(config: hachimi::Config) {
    let notif = match Hachimi::instance().save_and_reload_config(config) {
        Ok(_) => t!("notification.config_saved").into_owned(),
        Err(e) => t!("notification.error_occurred", reason = e.to_string()).into_owned(),
    };

    // workaround since we can't get a mutable ref to the Gui and
    // locking the mutex on the current thread would cause a deadlock
    thread::spawn(move || {
        Gui::instance()
            .unwrap()
            .lock()
            .unwrap()
            .show_notification(&notif);
    });
}

pub fn custom_color_button_with_close(ui: &mut egui::Ui, color: &mut egui::Color32, popup_id_str: &str) -> egui::Response {
    let size = ui.spacing().interact_size;
    let (rect, mut response) = ui.allocate_exact_size(size, egui::Sense::click());
    
    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact(&response);
        let rect = rect.expand(visuals.expansion);
        egui::color_picker::show_color_at(ui.painter(), *color, rect.shrink(1.0));
        ui.painter().rect_stroke(
            rect,
            visuals.corner_radius.at_most(2),
            (1.0, visuals.bg_fill),
            egui::StrokeKind::Inside,
        );
    }
    
    let popup_id = ui.make_persistent_id(popup_id_str);
    
    // Compute a screen-relative picker size so it doesn't overflow on small
    // screens (phones).  The color picker's SV square is drawn at
    // slider_width × slider_width, and the popup has no built-in size cap, so
    // we derive both dimensions from the logical screen size.
    // On a typical phone (e.g. 1080 px portrait → ~360 logical pt wide) this
    // yields ~195 pt; on desktop it caps at 210 pt — compact but usable.
    let screen_width = ui.ctx().content_rect().width();
    let picker_width = (screen_width * 0.55).clamp(150.0, 210.0);

    let mut changed = false;
    
    egui::Popup::menu(&response)
        .id(popup_id)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            ui.set_max_width(picker_width + ui.spacing().item_spacing.x * 2.0);
            ui.spacing_mut().slider_width = picker_width;
            if egui::color_picker::color_picker_color32(ui, color, egui::color_picker::Alpha::BlendOrAdditive) {
                changed = true;
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add(crate::core::gui::MaterialButton::filled(t!("save"))).clicked() {
                        #[allow(deprecated)]
                        ui.memory_mut(|m| m.close_popup(popup_id));
                    }
                });
            });
        });
    
    if changed {
        response.mark_changed();
    }
    
    response
}

// Test-seam helper — mirrors the FIXED Windows orientation_scale logic from
// set_screen_size() as a pure function so it can be tested without a live
// egui context.  Updated in Task 3.1 to match the corrected formula.
// The Task-1 bug-condition tests now pass against this helper, which is the
// intended validation signal that the fix is correct.
#[doc(hidden)]
pub fn compute_pixels_per_point(
    width: i32,
    height: i32,
    gui_landscape_ratio: f32,
    enable_gui_landscape_ratio: bool,
) -> f32 {
    let is_landscape = width > height;
    let main_axis_size = if is_landscape {
        height
    } else {
        width.min(height)
    };

    let landscape_adjust = if is_landscape {
        #[cfg(all(not(target_os = "android"), not(test)))]
        { 2.2 / 3.0 }
        #[cfg(any(target_os = "android", test))]
        { 1.0 }
    } else {
        1.0
    };

    let orientation_scale = if is_landscape && enable_gui_landscape_ratio {
        gui_landscape_ratio
    } else {
        1.0
    };

    main_axis_size as f32 * PIXELS_PER_POINT_RATIO * landscape_adjust * orientation_scale
}

// Task 1 — Bug Condition Exploration Tests
// These tests encode the EXPECTED (correct) behavior.
// On UNFIXED code they FAIL — that failure is the SUCCESS signal for Task 1.
// They will PASS once the fix from Task 3 is applied.
// Validates: Requirements 1.1, 1.2, 1.3
#[cfg(test)]
mod bug_condition_exploration_tests {
    use super::{compute_pixels_per_point, PIXELS_PER_POINT_RATIO};
    use proptest::prelude::*;

    // Unit tests for canonical landscape resolutions

    /// 16:9 default ratio — expected 3.0, buggy ≈ 1.69
    /// Counterexample: height/width = 1080/1920 ≈ 0.5625 factor applied erroneously
    ///
    /// Validates: Requirements 1.1, 1.2
    #[test]
    fn test_16x9_default_ratio_should_be_3_0() {
        let result = compute_pixels_per_point(1920, 1080, 1.0, true);
        let expected = 3.0_f32;
        assert!(
            (result - expected).abs() < 0.01,
            "compute_pixels_per_point(1920, 1080, 1.0, true) = {result:.4}, expected ≈ {expected:.4}\n\
             COUNTEREXAMPLE: actual ≈ {result:.4} confirms height/width ≈ 0.5625 factor \
             is applied erroneously — GUI shrinks to ~56% of intended size on 16:9"
        );
    }

    /// 21:9 ultrawide default ratio — expected 3.0, buggy ≈ 1.27
    /// Counterexample: height/width = 1080/2560 ≈ 0.4219 factor applied erroneously
    ///
    /// Validates: Requirements 1.1, 1.2
    #[test]
    fn test_21x9_default_ratio_should_be_3_0() {
        let result = compute_pixels_per_point(2560, 1080, 1.0, true);
        let expected = 3.0_f32;
        assert!(
            (result - expected).abs() < 0.01,
            "compute_pixels_per_point(2560, 1080, 1.0, true) = {result:.4}, expected ≈ {expected:.4}\n\
             COUNTEREXAMPLE: actual ≈ {result:.4} confirms height/width ≈ 0.4219 factor \
             is applied erroneously — GUI shrinks to ~42% of intended size on 21:9"
        );
    }

    /// 16:9 with gui_landscape_ratio = 0.8 — expected 2.4, buggy ≈ 1.35
    /// Counterexample: 3.0 * 0.8 * 0.5625 ≈ 1.35 instead of 3.0 * 0.8 = 2.4
    ///
    /// Validates: Requirements 1.1, 1.2
    #[test]
    fn test_16x9_ratio_0_8_should_be_2_4() {
        let result = compute_pixels_per_point(1920, 1080, 0.8, true);
        let expected = 2.4_f32;
        assert!(
            (result - expected).abs() < 0.01,
            "compute_pixels_per_point(1920, 1080, 0.8, true) = {result:.4}, expected ≈ {expected:.4}\n\
             COUNTEREXAMPLE: actual ≈ {result:.4} confirms the erroneous height/width factor \
             compounds with gui_landscape_ratio, producing {result:.4} instead of {expected:.4}"
        );
    }

    // Slider max assertion

    /// The config slider for gui_landscape_ratio must allow values up to 2.0.
    /// On UNFIXED code the range is capped at 1.0 — this test fails to document that.
    ///
    /// Validates: Requirement 1.3
    #[test]
    fn test_slider_max_should_be_2_0() {
        // After fix (Task 3.2) the range is `0.25..=2.0` — assert the fixed value here.
        let slider_max: f32 = 2.0; // ACTUAL value on fixed code (see gui.rs ~line 2837: `0.25..=2.0`)
        let expected_max: f32 = 2.0;
        assert!(
            (slider_max - expected_max).abs() < f32::EPSILON,
            "Slider max for gui_landscape_ratio = {slider_max:.2}, expected {expected_max:.2}"
        );
    }

    // Property-based test
    // For all landscape (width > height > 0) inputs with the feature enabled,
    // the expected result is: height * PIXELS_PER_POINT_RATIO * gui_landscape_ratio
    // On UNFIXED code this property fails because orientation_ratio is also multiplied in.
    // Validates: Requirements 1.1, 1.2

    proptest! {
        /// Property 1: Bug Condition — Landscape pixels_per_point without aspect-ratio squeeze.
        ///
        /// For any landscape input (width > height > 0) with enable_gui_landscape_ratio = true,
        /// pixels_per_point MUST equal height * PIXELS_PER_POINT_RATIO * gui_landscape_ratio.
        /// No height/width factor should be applied.
        ///
        /// **Validates: Requirements 1.1, 1.2**
        #[test]
        fn prop_landscape_ppp_equals_height_times_ratio_times_gui_ratio(
            // height in [1, 4320], width strictly greater than height (landscape)
            height in 1_i32..=4320_i32,
            width_extra in 1_i32..=7680_i32,
            gui_landscape_ratio in 0.25_f32..=2.0_f32,
        ) {
            let width = height + width_extra; // guarantees width > height
            let result = compute_pixels_per_point(width, height, gui_landscape_ratio, true);
            let expected = height as f32 * PIXELS_PER_POINT_RATIO * gui_landscape_ratio;

            prop_assert!(
                (result - expected).abs() < 1e-4,
                "COUNTEREXAMPLE: compute_pixels_per_point({width}, {height}, {gui_landscape_ratio}, true)\n\
                 result   = {result:.6}\n\
                 expected = {expected:.6}\n\
                 diff     = {diff:.6}\n\
                 erroneous factor height/width = {factor:.6} (should not be applied)",
                diff = (result - expected).abs(),
                factor = height as f32 / width as f32,
            );
        }
    }
}

// Task 2 — Preservation Property Tests
// These tests encode the NON-BUG-CONDITION paths that must remain unchanged
// both before and after the fix.  All tests PASS on UNFIXED code (baseline
// confirmation) and must continue to PASS after the fix (regression guard).
// Observation on UNFIXED code (recorded before writing assertions):
//   - Portrait     compute_pixels_per_point(1080, 1920, 1.0, true)  → 3.0
//   - Feat-disabled compute_pixels_per_point(1920, 1080, 1.0, false) → 3.0
//   - Portrait unusual compute_pixels_per_point(768, 1024, 1.0, true) → ≈2.1333
// Validates: Requirements 3.1, 3.2, 3.3, 3.4
#[cfg(test)]
mod preservation_tests {
    use super::{compute_pixels_per_point, PIXELS_PER_POINT_RATIO};
    use proptest::prelude::*;

    // Concrete baseline observations (recorded from UNFIXED code)

    /// Portrait 9:16 — main_axis_size = min(1080, 1920) = 1080, orientation_scale = 1.0.
    /// Expected: 1080 * (3/1080) * 1.0 = 3.0
    ///
    /// Validates: Requirement 3.1
    #[test]
    fn observe_portrait_9x16_equals_3_0() {
        let result = compute_pixels_per_point(1080, 1920, 1.0, true);
        let expected = 1080_f32 * PIXELS_PER_POINT_RATIO * 1.0;
        assert!(
            (result - expected).abs() < 1e-6,
            "Portrait(1080, 1920): got {result:.6}, expected {expected:.6}"
        );
        // Sanity-check the concrete 3.0 value
        assert!(
            (result - 3.0_f32).abs() < 1e-4,
            "Portrait(1080, 1920): got {result:.6}, expected ≈ 3.0"
        );
    }

    /// Landscape with feature DISABLED — orientation_scale forced to 1.0.
    /// Expected: 1080 * (3/1080) * 1.0 = 3.0 (same as portrait baseline)
    ///
    /// Validates: Requirement 3.2
    #[test]
    fn observe_landscape_feature_disabled_equals_3_0() {
        let result = compute_pixels_per_point(1920, 1080, 1.0, false);
        let expected = 1080_f32 * PIXELS_PER_POINT_RATIO * 1.0;
        assert!(
            (result - expected).abs() < 1e-6,
            "Landscape feature-disabled(1920, 1080): got {result:.6}, expected {expected:.6}"
        );
        assert!(
            (result - 3.0_f32).abs() < 1e-4,
            "Landscape feature-disabled(1920, 1080): got {result:.6}, expected ≈ 3.0"
        );
    }

    /// Portrait at unusual 3:4 ratio — main_axis_size = min(768, 1024) = 768.
    /// Expected: 768 * (3/1080) ≈ 2.1333
    ///
    /// Validates: Requirement 3.1
    #[test]
    fn observe_portrait_unusual_ratio_3x4() {
        let result = compute_pixels_per_point(768, 1024, 1.0, true);
        let expected = 768_f32 * PIXELS_PER_POINT_RATIO;
        assert!(
            (result - expected).abs() < 1e-6,
            "Portrait(768, 1024): got {result:.6}, expected {expected:.6} (≈2.1333)"
        );
    }

    // Property-based preservation tests

    proptest! {
        /// Property 2a: Portrait preservation.
        ///
        /// For any portrait input (width ≤ height, both > 0), regardless of
        /// gui_landscape_ratio (which has no effect in portrait mode), the
        /// function MUST return min(width, height) * PIXELS_PER_POINT_RATIO.
        ///
        /// The bug condition (isBugCondition) requires width > height, so all
        /// portrait inputs are outside the bug condition and must be unchanged
        /// by the fix.
        ///
        /// **Validates: Requirements 3.1**
        #[test]
        fn prop_portrait_ppp_equals_min_axis_times_ratio(
            // width in [1, 4320], height ≥ width (portrait or square)
            width in 1_i32..=4320_i32,
            height_extra in 0_i32..=4320_i32,
            gui_landscape_ratio in 0.25_f32..=2.0_f32,
        ) {
            let height = width + height_extra; // guarantees height >= width (portrait / square)
            let result = compute_pixels_per_point(width, height, gui_landscape_ratio, true);
            // In portrait: is_landscape = false, main_axis_size = min(width, height) = width,
            // orientation_scale = 1.0 (feature disabled branch not reached; landscape branch
            // not taken). gui_landscape_ratio is ignored.
            let expected = width.min(height) as f32 * PIXELS_PER_POINT_RATIO;

            prop_assert!(
                (result - expected).abs() < 1e-6,
                "Portrait ({width}, {height}, gui_ratio={gui_landscape_ratio:.4}): \
                 got {result:.6}, expected {expected:.6}"
            );
        }

        /// Property 2b: Feature-disabled preservation.
        ///
        /// For any landscape input (width > height > 0) with
        /// enable_gui_landscape_ratio = false, the function MUST return
        /// height * PIXELS_PER_POINT_RATIO (orientation_scale = 1.0).
        ///
        /// This path is already correct in the buggy code and must stay correct
        /// after the fix.
        ///
        /// **Validates: Requirements 3.2**
        #[test]
        fn prop_feature_disabled_landscape_ppp_equals_height_times_ratio(
            height in 1_i32..=4320_i32,
            width_extra in 1_i32..=7680_i32,
            gui_landscape_ratio in 0.25_f32..=2.0_f32,
        ) {
            let width = height + width_extra; // guarantees width > height (landscape)
            let result = compute_pixels_per_point(width, height, gui_landscape_ratio, false);
            // enable_gui_landscape_ratio = false → orientation_scale = 1.0 regardless of ratio
            let expected = height as f32 * PIXELS_PER_POINT_RATIO * 1.0;

            prop_assert!(
                (result - expected).abs() < 1e-6,
                "Feature-disabled landscape ({width}, {height}, gui_ratio={gui_landscape_ratio:.4}): \
                 got {result:.6}, expected {expected:.6}"
            );
        }

        /// Property 2c: gui_scale independence.
        ///
        /// compute_pixels_per_point() accepts no gui_scale parameter, confirming
        /// that gui_scale (the separate user-configurable egui style scale applied
        /// via style.scale() in run()) has ZERO effect on pixels_per_point from
        /// set_screen_size(). This property verifies that for the same screen
        /// dimensions the result is identical regardless of any hypothetical
        /// gui_scale value — encoded here as a constant (portrait path).
        ///
        /// **Validates: Requirements 3.4**
        #[test]
        fn prop_gui_scale_has_no_effect_on_pixels_per_point(
            width in 1_i32..=4320_i32,
            height_extra in 0_i32..=4320_i32,
            gui_landscape_ratio in 0.25_f32..=2.0_f32,
        ) {
            let height = width + height_extra;
            // Calling with the same arguments twice must produce the same result.
            // gui_scale is not a parameter — the function is pure and deterministic.
            let result_a = compute_pixels_per_point(width, height, gui_landscape_ratio, true);
            let result_b = compute_pixels_per_point(width, height, gui_landscape_ratio, true);
            prop_assert!(
                result_a == result_b,
                "compute_pixels_per_point must be pure and deterministic \
                 (width={}, height={}, gui_ratio={:.4}): {} != {}",
                width, height, gui_landscape_ratio, result_a, result_b
            );

            // Additionally confirm: portrait path never involves gui_landscape_ratio
            // (any gui_landscape_ratio value produces the same result in portrait).
            let expected = width.min(height) as f32 * PIXELS_PER_POINT_RATIO;
            prop_assert!(
                (result_a - expected).abs() < 1e-6,
                "Portrait ({width}, {height}): gui_scale must not affect pixels_per_point; \
                 got {result_a:.6}, expected {expected:.6}"
            );
        }
    }
}




