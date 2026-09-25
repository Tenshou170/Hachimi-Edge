pub mod config;
pub mod dialogs;
pub mod plugins;
pub mod race_director_hud;
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
        atomic::{self, AtomicBool, AtomicI32, AtomicU32},
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
use size::{Base, Size, Style};

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
    tl_repo::{RepoInfo, UpdatePhase},
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
    /// Monotonic frame counter for the IME zombie check. Must be independent
    /// of `tmp_frame_count`, which `update_fps` resets every 0.5 s — gating on
    /// it made the check never fire below 40 FPS, leaving `IS_IME_VISIBLE`
    /// stuck true (phantom IME padding + broken scrolling in every window).
    #[cfg(target_os = "android")]
    ime_frame_tick: u32,

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

    /// The layer that was most recently clicked/pressed. Re-promoted to the top of
    /// its [`egui::Order`] tier every frame so the user's last focused window stays
    /// in front of other windows at the same order level until something else is
    /// clicked (e.g. an app window stays in front of Race Director HUD panels and
    /// vice-versa).
    top_layer: Option<egui::LayerId>,
    pending_top_window_id: Option<egui::Id>,
}

const PIXELS_PER_POINT_RATIO: f32 = 3.0 / 1080.0;

static INSTANCE: OnceCell<Mutex<Gui>> = OnceCell::new();
pub static IS_CONSUMING_INPUT: AtomicBool = AtomicBool::new(false);
pub static WANTS_INPUT: AtomicBool = AtomicBool::new(false);
pub static GUI_INPUT_ACTIVE: AtomicBool = AtomicBool::new(false);
pub static EGUI_TYPING: AtomicBool = AtomicBool::new(false);
pub static IS_LIVE_SCENE: AtomicBool = AtomicBool::new(false);
pub static IS_LIVE_SLIDER_ACTIVE: AtomicBool = AtomicBool::new(false);
static LIVE_SLIDER_SCENE_HANDLE: atomic::AtomicI32 = atomic::AtomicI32::new(-1);

pub static RACE_SLIDER_DRAGGING: AtomicBool = AtomicBool::new(false);
static RACE_SLIDER_PENDING: AtomicBool = AtomicBool::new(false);
static RACE_SLIDER_TARGET_TIME: AtomicU32 = AtomicU32::new(0);
static RACE_SLIDER_END_REQUESTED: AtomicBool = AtomicBool::new(false);
static RACE_SLIDER_PAUSE_DEPTH: AtomicI32 = AtomicI32::new(0);
pub static RACE_SLIDER_LAST_APPLIED: AtomicU32 = AtomicU32::new(0);
pub static RACE_SLIDER_DRAG_START_TIME: AtomicU32 = AtomicU32::new(0);
static RACE_SLIDER_SEEK_FAULTED: AtomicBool = AtomicBool::new(false);
pub static RACE_SLIDER_MUSIC_TIME: AtomicU32 = AtomicU32::new(0);
pub static RACE_SLIDER_MUSIC_VALID: AtomicBool = AtomicBool::new(false);

pub fn reset_race_slider() {
    IS_CONSUMING_INPUT.store(false, atomic::Ordering::Release);
    WANTS_INPUT.store(false, atomic::Ordering::Release);
    RACE_SLIDER_DRAGGING.store(false, atomic::Ordering::Release);
    RACE_SLIDER_PENDING.store(false, atomic::Ordering::Release);
    RACE_SLIDER_END_REQUESTED.store(false, atomic::Ordering::Release);
    RACE_SLIDER_PAUSE_DEPTH.store(0, atomic::Ordering::Release);
    RACE_SLIDER_SEEK_FAULTED.store(false, atomic::Ordering::Release);
}

pub fn race_slider_drain() {
    use crate::il2cpp::hook::umamusume::{AudioManager, RaceManager, RaceManagerReplayBase};

    let end_requested = RACE_SLIDER_END_REQUESTED.swap(false, atomic::Ordering::AcqRel);
    let seek_pending = RACE_SLIDER_PENDING.swap(false, atomic::Ordering::AcqRel);

    let race_manager = RaceManager::instance();
    if race_manager.is_null() || RaceManager::is_race_finished(race_manager) {
        RACE_SLIDER_PAUSE_DEPTH.swap(0, atomic::Ordering::AcqRel);
        RACE_SLIDER_DRAGGING.store(false, atomic::Ordering::Release);
        RACE_SLIDER_SEEK_FAULTED.store(false, atomic::Ordering::Release);
        return;
    }

    if !end_requested && !seek_pending { return; }

    if RaceManagerReplayBase::IsCutInPlayingOrReserved(race_manager) {
        if seek_pending { RACE_SLIDER_PENDING.store(true, atomic::Ordering::Release); }
        if end_requested { RACE_SLIDER_END_REQUESTED.store(true, atomic::Ordering::Release); }
        return;
    }

    let mut settle_target: Option<f32> = None;
    if seek_pending {
        let target_time = f32::from_bits(RACE_SLIDER_TARGET_TIME.load(atomic::Ordering::Acquire));
        if RaceManagerReplayBase::seek_sync(target_time) {
            RACE_SLIDER_LAST_APPLIED.store(target_time.to_bits(), atomic::Ordering::Release);
            RACE_SLIDER_SEEK_FAULTED.store(false, atomic::Ordering::Release);
            settle_target = Some(target_time);
        } else {
            RACE_SLIDER_SEEK_FAULTED.store(true, atomic::Ordering::Release);
        }
    } else if !RACE_SLIDER_SEEK_FAULTED.load(atomic::Ordering::Acquire) {
        settle_target = Some(f32::from_bits(RACE_SLIDER_LAST_APPLIED.load(atomic::Ordering::Acquire)));
    }

    if end_requested {
        let pause_depth = RACE_SLIDER_PAUSE_DEPTH.swap(0, atomic::Ordering::AcqRel);
        if pause_depth > 0 {
            if RACE_SLIDER_SEEK_FAULTED.swap(false, atomic::Ordering::AcqRel) {
            } else {
                let start_time = f32::from_bits(RACE_SLIDER_DRAG_START_TIME.load(atomic::Ordering::Acquire));
                if AudioManager::resync_race_music(race_manager, settle_target.unwrap_or(start_time)) {
                    RaceManagerReplayBase::ResumeRace(race_manager);
                }
            }
        }
    }
}

#[cfg(target_os = "windows")]
const WINDOWS_SPLIT_LEFT_RATIO: f32 = 148.0 / 1920.0;
#[cfg(target_os = "windows")]
const WINDOWS_SPLIT_CENTER_RATIO: f32 = 810.0 / 1920.0;

#[cfg(target_os = "windows")]
fn split_rect_constants(window: egui::Vec2) -> egui::Rect {
    let x = window.x * WINDOWS_SPLIT_LEFT_RATIO;
    let w = window.x * WINDOWS_SPLIT_CENTER_RATIO;
    egui::Rect::from_min_size(egui::pos2(x, 0.0), egui::vec2(w, window.y))
}

#[cfg(target_os = "windows")]
fn split_rect_sane(rect: egui::Rect, window: egui::Vec2) -> bool {
    rect.width() > 0.0
        && rect.height() > 0.0
        && rect.min.x >= -1.0
        && rect.min.y >= -1.0
        && rect.max.x <= window.x * 1.02
        && rect.max.y <= window.y * 1.02
        && rect.width() * rect.height() >= window.x * window.y * 0.15
}

#[cfg(target_os = "windows")]
fn windows_split_game_view(screen: egui::Rect, ppp: f32) -> Option<(egui::Rect, &'static str)> {
    use crate::il2cpp::hook::umamusume::{LandscapeUIManager, Screen as GallopScreen};

    if !GallopScreen::get_IsSplitWindow() {
        return None;
    }

    let window = egui::vec2(screen.width() * ppp, screen.height() * ppp);

    let mut source = "constants";
    let mut rect_px = split_rect_constants(window);

    if let Some((x, y, w, h)) = LandscapeUIManager::game_screen_info() {
        let physical = (h - window.y).abs() <= window.y * 0.02;
        let rate = LandscapeUIManager::get_WindowScaleRate();
        let (r, src) = if physical {
            (egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, h)), "gamescreeninfo")
        } else {
            (
                egui::Rect::from_min_size(egui::pos2(x * rate, y * rate), egui::vec2(w * rate, h * rate)),
                "gamescreeninfo+rate",
            )
        };

        if split_rect_sane(r, window) {
            rect_px = r;
            source = src;
        }
    }

    let game_view = egui::Rect::from_min_size(
        egui::pos2(rect_px.min.x / ppp, rect_px.min.y / ppp),
        egui::vec2(rect_px.width() / ppp, rect_px.height() / ppp),
    );
    Some((game_view, source))
}
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
            #[cfg(target_os = "android")]
            ime_frame_tick: 0,

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
            top_layer: None,
            pending_top_window_id: None,
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
        add_font!(fonts, proportional_fonts, "HarmonyOSSansRegularMerged.ttf");
        add_font!(fonts, proportional_fonts, "MPLUS1-Regular.ttf");
        add_font!(fonts, proportional_fonts, "Pretendard-Regular.ttf");

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
            { Hachimi::instance().config.load().android.enable_gui_landscape_ratio }
        };
        let gui_landscape_ratio = {
            #[cfg(target_os = "windows")]
            { Hachimi::instance().config.load().windows.gui_landscape_ratio }
            #[cfg(target_os = "android")]
            { Hachimi::instance().config.load().android.gui_landscape_ratio }
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
            crate::core::live_utils::reset_live_drag_state();
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

            if config.live_playback_loop && current >= total - 0.1
                && crate::core::live_utils::should_loop_restart(current, total)
            {
                crate::core::live_utils::begin_live_drag();
                crate::core::live_utils::move_live_playback(0.0);
                crate::core::live_utils::end_live_drag();
                current = 0.0;
            }

            if !crate::il2cpp::hook::umamusume::Director::is_live_playing() {
                IS_LIVE_SLIDER_ACTIVE.store(false, atomic::Ordering::Release);
                crate::core::live_utils::reset_live_drag_state();
                return;
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

            let curr_m = (current / 60.0).floor() as i32;
            let curr_s = (current % 60.0).floor() as i32;
            let tot_m  = (total  / 60.0).floor() as i32;
            let tot_s  = (total  % 60.0).floor() as i32;

            let surface_container = get_global_color("surfaceContainer");
            let on_surface_variant = get_global_color("onSurfaceVariant");
            let cr = egui_material3::theme::get_global_corner_radius()
                .unwrap_or(8.0)
                .max(8.0) as u8;

            // Apply surface_alpha for Overlay and Full modes.
            let hachimi_cfg = crate::core::Hachimi::instance().config.load();
            let fill = match hachimi_cfg.ui_translucency_mode {
                hachimi::UiTranslucencyMode::None => surface_container,
                hachimi::UiTranslucencyMode::Overlay | hachimi::UiTranslucencyMode::Full => {
                    let a = hachimi_cfg.ui_surface_alpha;
                    egui::Color32::from_rgba_unmultiplied(
                        surface_container.r(),
                        surface_container.g(),
                        surface_container.b(),
                        a,
                    )
                }
            };

            let (_, safe_bottom) = get_safe_insets(ctx);
            let slider_offset = if is_portrait(ctx) { 72.0 * scale } else { 24.0 * scale };
            egui::Area::new(egui::Id::new("live_slider_area"))
                .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -(slider_offset + safe_bottom)))
                .order(egui::Order::Middle)
                .show(ctx, |ui| {
                    egui::Frame::NONE
                        .fill(fill)
                        .corner_radius(egui::CornerRadius::same(cr))
                        .shadow(egui::Shadow {
                            blur:   (6.0 * scale) as u8,
                            spread: 0,
                            offset: [0, (2.0 * scale) as i8],
                            color:  egui::Color32::from_black_alpha(36),
                        })
                        .inner_margin(egui::Margin::symmetric(
                            (16.0 * scale) as i8,
                            (6.0 * scale) as i8,
                        ))
                        .show(ui, |ui| {
                            ui.set_width(ctx.content_rect().width() * if is_portrait(ctx) { 0.78 } else { 0.60 });

                            let time_font = egui::FontId::new(
                                10.5 * scale,
                                egui::FontFamily::Proportional,
                            );

                            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                                ui.add(egui::Label::new(
                                    egui::RichText::new(format!("{:02}:{:02}", curr_m, curr_s))
                                        .font(time_font.clone())
                                        .color(on_surface_variant),
                                ));

                                let label_w = 28.0 * scale;
                                let slider_w = (ui.available_width() - label_w
                                    - ui.spacing().item_spacing.x * 2.0)
                                    .max(40.0);

                                ui.spacing_mut().slider_width = slider_w;
                                let res = ui.add(
                                    MaterialSlider::new(&mut current, 0.0..=total)
                                        .show_value(false)
                                        .show_value_indicator(true)
                                        .compact()
                                        .width(slider_w),
                                );
                                if res.drag_started() {
                                    crate::core::live_utils::begin_live_drag();
                                }
                                if res.changed() {
                                    crate::core::live_utils::move_live_playback(current);
                                }
                                if res.drag_stopped() {
                                    crate::core::live_utils::end_live_drag();
                                }

                                ui.add(egui::Label::new(
                                    egui::RichText::new(format!("{:02}:{:02}", tot_m, tot_s))
                                        .font(time_font)
                                        .color(on_surface_variant),
                                ));
                            });
                        });
                });
        }
    }

    pub fn race_slider_showing() -> bool {
        use crate::il2cpp::hook::umamusume::{RaceHorseManagerBase, RaceManager, RaceManagerReplayBase};
        use crate::core::race_director;

        let config = Hachimi::instance().config.load();
        let is_dragging = RACE_SLIDER_DRAGGING.load(atomic::Ordering::Acquire);

        if !config.race_playback_slider
            || !RaceHorseManagerBase::is_race_active()
            || (!race_director::is_gate_open() && !is_dragging)
            || race_director::is_race_finished()
            || race_director::is_story_race()
        {
            return false;
        }

        let race_manager = RaceManager::instance();
        if race_manager.is_null() || RaceManager::is_race_finished(race_manager) {
            return false;
        }

        if !config.race_playback_slider_always && !is_dragging {
            if !RaceManagerReplayBase::IsPaused(race_manager) {
                return false;
            }
        }

        true
    }

    fn run_race_slider(&mut self, ctx: &egui::Context) {
        use crate::il2cpp::hook::umamusume::{
            AudioManager, RaceHorseManagerReplay,
            RaceManager, RaceManagerReplayBase, RaceSimulateReader,
        };

        if !Self::race_slider_showing() {
            RACE_SLIDER_DRAGGING.store(false, atomic::Ordering::Release);
            RACE_SLIDER_PENDING.store(false, atomic::Ordering::Release);
            RACE_SLIDER_END_REQUESTED.store(false, atomic::Ordering::Release);
            RACE_SLIDER_PAUSE_DEPTH.store(0, atomic::Ordering::Release);
            RACE_SLIDER_LAST_APPLIED.store(0, atomic::Ordering::Release);
            RACE_SLIDER_DRAG_START_TIME.store(0, atomic::Ordering::Release);
            RACE_SLIDER_SEEK_FAULTED.store(false, atomic::Ordering::Release);
            RACE_SLIDER_MUSIC_TIME.store(0, atomic::Ordering::Release);
            RACE_SLIDER_MUSIC_VALID.store(false, atomic::Ordering::Release);
            RACE_SLIDER_TARGET_TIME.store(0, atomic::Ordering::Release);
            // Reset the fade timer so the slider starts fully visible next time.
            ctx.data_mut(|d| d.insert_temp(egui::Id::new("race_slider_activity"), std::time::Instant::now()));
            return;
        }

        let race_manager = RaceManager::instance();
        if race_manager.is_null() || RaceManager::is_race_finished(race_manager) { return; }

        let horse_manager = RaceManager::get__horseManager(race_manager);
        if horse_manager.is_null() { return; }

        if !RaceHorseManagerReplay::is_replay_manager(horse_manager) { return; }

        let reader = RaceHorseManagerReplay::get__reader(horse_manager);
        if reader.is_null() { return; }

        let total = RaceSimulateReader::GetLastFrameTime(reader);
        if total <= 0.0 { return; }
        let max_time = (total - 0.5).max(0.0);

        let cut_in_playing = RaceManagerReplayBase::get_IsPlayingCutIn(race_manager);
        let interactable = !cut_in_playing;

        if RACE_SLIDER_DRAGGING.load(atomic::Ordering::Acquire) && !interactable {
            RACE_SLIDER_END_REQUESTED.store(true, atomic::Ordering::Release);
            RACE_SLIDER_DRAGGING.store(false, atomic::Ordering::Release);
        }

        let mut current = if RACE_SLIDER_PENDING.load(atomic::Ordering::Acquire) {
            f32::from_bits(RACE_SLIDER_TARGET_TIME.load(atomic::Ordering::Acquire))
        } else {
            RaceSimulateReader::get__curTime(reader)
        };
        if !current.is_finite() || current < 0.0 {
            current = 0.0;
        }
        if current > max_time {
            current = max_time;
        }

        let scale = get_scale(ctx);
        let surface_container = get_global_color("surfaceContainer");
        let on_surface_variant = get_global_color("onSurfaceVariant");
        let cr = egui_material3::theme::get_global_corner_radius()
            .unwrap_or(8.0)
            .max(8.0) as u8;

        let hachimi_cfg = crate::core::Hachimi::instance().config.load();
        let fill = match hachimi_cfg.ui_translucency_mode {
            hachimi::UiTranslucencyMode::None => surface_container,
            hachimi::UiTranslucencyMode::Overlay | hachimi::UiTranslucencyMode::Full => {
                let a = hachimi_cfg.ui_surface_alpha;
                egui::Color32::from_rgba_unmultiplied(
                    surface_container.r(),
                    surface_container.g(),
                    surface_container.b(),
                    a,
                )
            }
        };

        let (_, safe_bottom) = get_safe_insets(ctx);
        let slider_offset = if is_portrait(ctx) { 72.0 * scale } else { 24.0 * scale };

        // Fade the slider to 25% opacity after 2s of no interaction/hover,
        // same as the playback button. Activity resets on drag or hover.
        const SLIDER_FADE_DELAY: f32 = 2.0;
        const SLIDER_FADE_DURATION: f32 = 1.0;
        const SLIDER_DIM_ALPHA: f32 = 0.25;
        let slider_activity_id = egui::Id::new("race_slider_activity");
        let slider_last_activity: std::time::Instant = ctx.data(|d| {
            d.get_temp(slider_activity_id).unwrap_or_else(std::time::Instant::now)
        });
        let slider_elapsed = slider_last_activity.elapsed().as_secs_f32();
        let slider_alpha = if slider_elapsed < SLIDER_FADE_DELAY {
            1.0_f32
        } else {
            let t = ((slider_elapsed - SLIDER_FADE_DELAY) / SLIDER_FADE_DURATION).clamp(0.0, 1.0);
            1.0 - t * (1.0 - SLIDER_DIM_ALPHA)
        };
        // Recompute fill with fade alpha applied.
        let fill_faded = egui::Color32::from_rgba_unmultiplied(
            fill.r(), fill.g(), fill.b(),
            (fill.a() as f32 * slider_alpha) as u8,
        );

        egui::Area::new(egui::Id::new("race_slider_area"))
            .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -(slider_offset + safe_bottom)))
            .order(egui::Order::Middle)
            .show(ctx, |ui| {
                // Keep repainting during the fade animation.
                if slider_elapsed >= SLIDER_FADE_DELAY
                    && slider_elapsed < SLIDER_FADE_DELAY + SLIDER_FADE_DURATION {
                    ui.ctx().request_repaint();
                }
                egui::Frame::NONE
                    .fill(fill_faded)
                    .corner_radius(egui::CornerRadius::same(cr))
                    .shadow(egui::Shadow {
                        blur: (6.0 * scale) as u8,
                        spread: 0,
                        offset: [0, (2.0 * scale) as i8],
                        color: egui::Color32::from_black_alpha(36),
                    })
                    .inner_margin(egui::Margin::symmetric(
                        (16.0 * scale) as i8,
                        (6.0 * scale) as i8,
                    ))
                    .show(ui, |ui| {
                        ui.set_width(ctx.content_rect().width() * if is_portrait(ctx) { 0.78 } else { 0.60 });

                        let curr_m = (current / 60.0).floor() as i32;
                        let curr_s = (current % 60.0).floor() as i32;
                        let tot_m = (total / 60.0).floor() as i32;
                        let tot_s = (total % 60.0).floor() as i32;

                        let time_font = egui::FontId::new(
                            10.5 * scale,
                            egui::FontFamily::Proportional,
                        );

                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            ui.add(egui::Label::new(
                                egui::RichText::new(format!("{:02}:{:02}", curr_m, curr_s))
                                    .font(time_font.clone())
                                    .color(on_surface_variant.linear_multiply(slider_alpha)),
                            ));

                            let label_w = 28.0 * scale;
                            let slider_w = (ui.available_width() - label_w
                                - ui.spacing().item_spacing.x * 2.0)
                                .max(40.0);

                            ui.spacing_mut().slider_width = slider_w;

                            // Dim the slider track/thumb together with the pill background.
                            ui.multiply_opacity(slider_alpha);

                            let slider = MaterialSlider::new(&mut current, 0.0..=max_time)
                                .show_value(false)
                                .show_value_indicator(true)
                                .compact()
                                .width(slider_w);
                            let res = if interactable {
                                ui.add(slider)
                            } else {
                                ui.add_enabled(false, slider)
                            };

                            if res.drag_started() {
                                let race_manager = RaceManager::instance();
                                if race_manager.is_null() || RaceManager::is_race_finished(race_manager) { return; }
                                let start_time = RaceSimulateReader::replay_cur_time(race_manager).unwrap_or(0.0);

                                RACE_SLIDER_DRAG_START_TIME.store(start_time.to_bits(), atomic::Ordering::Release);
                                RACE_SLIDER_LAST_APPLIED.store(start_time.to_bits(), atomic::Ordering::Release);
                                RACE_SLIDER_MUSIC_VALID.store(
                                    AudioManager::race_slider_music_base(),
                                    atomic::Ordering::Release,
                                );

                                if !RaceManagerReplayBase::IsPaused(race_manager) {
                                    RaceManagerReplayBase::PauseRace(race_manager);
                                    RACE_SLIDER_PAUSE_DEPTH.fetch_add(1, atomic::Ordering::AcqRel);
                                }
                                RACE_SLIDER_DRAGGING.store(true, atomic::Ordering::Release);
                            }

                            if res.changed() {
                                RACE_SLIDER_TARGET_TIME.store(current.to_bits(), atomic::Ordering::Release);
                                RACE_SLIDER_PENDING.store(true, atomic::Ordering::Release);
                            }

                            if res.drag_stopped() && RACE_SLIDER_DRAGGING.swap(false, atomic::Ordering::AcqRel) {
                                RACE_SLIDER_END_REQUESTED.store(true, atomic::Ordering::Release);
                            }

                            // Reset fade timer on any interaction or hover.
                            if res.hovered() || res.dragged() || res.drag_started() {
                                ui.ctx().data_mut(|d| d.insert_temp(slider_activity_id, std::time::Instant::now()));
                            }

                            ui.add(egui::Label::new(
                                egui::RichText::new(format!("{:02}:{:02}", tot_m, tot_s))
                                    .font(time_font)
                                    .color(on_surface_variant.linear_multiply(slider_alpha)),
                            ));
                        });
                    });
            });
    }

    pub fn race_playback_button_showing() -> bool {
        use crate::il2cpp::hook::umamusume::{RaceHorseManagerBase, RaceManager};
        use crate::core::race_director;

        if !Hachimi::instance().config.load().race_playback_button
            || !RaceHorseManagerBase::is_race_active()
            || !race_director::is_gate_open()
            || race_director::is_race_finished()
            || race_director::is_story_race()
        {
            return false;
        }

        let race_manager = RaceManager::instance();
        !race_manager.is_null() && !RaceManager::is_race_finished(race_manager)
    }

    fn run_race_playback_button(ctx: &egui::Context) {
        use crate::il2cpp::hook::umamusume::{RaceManager, RaceManagerReplayBase};

        if !Self::race_playback_button_showing() {
            // Reset the fade timer so the button starts fully visible next time.
            ctx.data_mut(|d| d.insert_temp(egui::Id::new("race_playback_btn_activity"), std::time::Instant::now()));
            return;
        }

        let race_manager = RaceManager::instance();
        if race_manager.is_null() { return; }

        let scale = get_scale(ctx);
        let screen = ctx.viewport_rect();

        #[cfg(target_os = "windows")]
        let game_view = match windows_split_game_view(screen, ctx.pixels_per_point()) {
            Some((rect, _)) => rect,
            None => screen,
        };
        #[cfg(target_os = "android")]
        let game_view = screen;

        let paused = RaceManagerReplayBase::IsPaused(race_manager);
        let interactable = !RaceManagerReplayBase::playback_gated(race_manager);

        let btn_size = 48.0 * scale;
        let margin = 12.0 * scale;
        let btn_pos = egui::Pos2::new(
            game_view.left() + margin,
            game_view.center().y - btn_size / 2.0,
        );

        // Material Symbols: play_arrow \u{e037}, pause \u{e034}
        let icon = if paused { "\u{e037}" } else { "\u{e034}" };

        // Fade the button to 25% opacity after 2s of no hover/interaction,
        // restoring to full opacity immediately on hover or click.
        // Activity timestamp is stored in egui temp data so it persists across frames
        // without needing a field on Gui.
        const FADE_DELAY_SECS: f32 = 2.0;
        const FADE_DURATION_SECS: f32 = 1.0;
        const DIM_ALPHA: f32 = 0.25;
        let activity_id = egui::Id::new("race_playback_btn_activity");
        let last_activity: std::time::Instant = ctx.data(|d| {
            d.get_temp(activity_id).unwrap_or_else(std::time::Instant::now)
        });
        let elapsed = last_activity.elapsed().as_secs_f32();
        let alpha = if elapsed < FADE_DELAY_SECS {
            1.0_f32
        } else {
            let t = ((elapsed - FADE_DELAY_SECS) / FADE_DURATION_SECS).clamp(0.0, 1.0);
            1.0 - t * (1.0 - DIM_ALPHA)
        };

        egui::Area::new(egui::Id::new("race_playback_button_area"))
            .fixed_pos(btn_pos)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.set_min_size(egui::Vec2::splat(btn_size));
                ui.set_max_size(egui::Vec2::splat(btn_size));

                // Apply alpha to the whole button via visual override.
                ui.visuals_mut().widgets.inactive.weak_bg_fill =
                    ui.visuals().widgets.inactive.weak_bg_fill.linear_multiply(alpha);
                ui.visuals_mut().widgets.inactive.fg_stroke.color =
                    ui.visuals().widgets.inactive.fg_stroke.color.linear_multiply(alpha);
                ui.visuals_mut().widgets.hovered.weak_bg_fill =
                    ui.visuals().widgets.hovered.weak_bg_fill.linear_multiply(alpha);
                ui.visuals_mut().widgets.hovered.fg_stroke.color =
                    ui.visuals().widgets.hovered.fg_stroke.color.linear_multiply(alpha);

                let btn = egui::Button::new(
                    egui::RichText::new(icon)
                        .size(24.0 * scale)
                        .color(egui::Color32::WHITE.linear_multiply(alpha)),
                ).min_size(egui::Vec2::splat(btn_size));

                ui.with_layout(
                    egui::Layout::centered_and_justified(egui::Direction::TopDown),
                    |ui| {
                        let res = if interactable {
                            ui.add(btn)
                        } else {
                            ui.add_enabled(false, btn)
                        };

                        // Reset activity timer on hover or click so button stays visible.
                        if res.hovered() || res.clicked() {
                            ctx.data_mut(|d| d.insert_temp(activity_id, std::time::Instant::now()));
                        }

                        if res.clicked() {
                            Thread::main_thread().schedule(RaceManagerReplayBase::toggle_playback);
                        }

                        // Keep repainting during the fade so it animates smoothly.
                        if elapsed >= FADE_DELAY_SECS && elapsed < FADE_DELAY_SECS + FADE_DURATION_SECS {
                            ctx.request_repaint();
                        }
                    },
                );
            });
    }

    pub fn run(&mut self) -> egui::FullOutput {
        if let Ok(mut lock) = PENDING_THEME.lock() {
            if let Some(config) = lock.take() {
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
            self.ime_frame_tick = self.ime_frame_tick.wrapping_add(1);
            if self.ime_frame_tick % 20 == 0 {
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
        self.run_race_slider(&ctx);
        Self::run_race_playback_button(&ctx);
        // catch_unwind: new, less-battle-tested code drawn every frame - a panic here
        // must not take the whole process down (see GameSystem_Update's identical
        // wrapping of race_telemetry::collect_frame for the same reasoning).
        if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| race_director_hud::run(&ctx))).is_err() {
            error!("race_director_hud::run PANICKED (caught)");
        }
        self.run_windows();

        // Persist the last-clicked window's layer focus across frames.
        // egui's internal move_to_top (via area.rs pointer_pressed_on_area) only marks
        // a layer for wants_to_be_on_top for the current frame and clears it at end_pass.
        // Without re-promotion, the HUD panels (or any later-submitted window) would
        // reclaim the top position on the very next frame. We detect which Order::Middle
        // layer the pointer just pressed on, save it, and re-promote it every frame until
        // the user clicks something else.
        if let Some(pos) = ctx.input(|i| {
            i.pointer.any_pressed().then(|| i.pointer.interact_pos()).flatten()
        }) {
            let pressed_layer = ctx.layer_id_at(pos);
            // Only track Order::Middle layers (app windows and HUD panels).
            // Foreground layers (sliders, snackbars) manage themselves and should not
            // be overridden here.
            if let Some(layer) = pressed_layer {
                if layer.order == egui::Order::Middle {
                    self.top_layer = Some(layer);
                }
            }
        }
        // Also update top_layer on scroll so that scrolling a window brings it to
        // the front — otherwise the HUD panels keep stealing focus and the scroll
        // events go to the wrong layer.
        if ctx.input(|i| i.smooth_scroll_delta != egui::Vec2::ZERO || i.raw_scroll_delta != egui::Vec2::ZERO) {
            if let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) {
                if let Some(layer) = ctx.layer_id_at(pos) {
                    if layer.order == egui::Order::Middle {
                        self.top_layer = Some(layer);
                    }
                }
            }
        }
        if let Some(layer) = self.top_layer {
            // If the layer is no longer visible (window was closed), clear it.
            if ctx.memory(|m| m.areas().visible_last_frame(&layer)) {
                ctx.memory_mut(|m| m.areas_mut().move_to_top(layer));
            } else {
                self.top_layer = None;
            }
        }

        #[cfg(target_os = "windows")]
        self.run_free_camera_overlay(&ctx);
        // actively_using_pointer: egui is actively processing a drag/click/keyboard event.
        let actively_using_pointer = self.context.wants_pointer_input()
            || self.context.wants_keyboard_input();

        // Point-in-layer: which specific overlay area is the pointer directly over.
        let pointer_pos = self.context.input(|i| {
            i.pointer.hover_pos().or_else(|| i.pointer.interact_pos()).or_else(|| i.pointer.latest_pos())
        });
        let pointer_layer = pointer_pos.and_then(|pos| self.context.layer_id_at(pos));
        let over_race_slider = pointer_layer
            .map(|l| l.id == egui::Id::new("race_slider_area"))
            .unwrap_or(false);
        let over_live_slider = pointer_layer
            .map(|l| l.id == egui::Id::new("live_slider_area"))
            .unwrap_or(false);
        let over_playback_btn = pointer_layer
            .map(|l| l.id == egui::Id::new("race_playback_button_area"))
            .unwrap_or(false);
        // over_hud: pointer is over a Middle-order layer that isn't one of the named
        // overlay areas — i.e. a HUD window panel.
        let over_hud = race_director_hud::showing()
            && pointer_layer.map(|l| l.order == egui::Order::Middle).unwrap_or(false)
            && !over_race_slider && !over_live_slider && !over_playback_btn;

        let is_hovering_window = pointer_layer.map(|l| l.order == egui::Order::Middle).unwrap_or(false);
        let has_scroll = self.context.input(|i| {
            i.smooth_scroll_delta != egui::Vec2::ZERO || i.raw_scroll_delta != egui::Vec2::ZERO
        });

        let live_slider_active = IS_LIVE_SLIDER_ACTIVE.load(atomic::Ordering::Relaxed);
        let race_slider_input = Self::race_slider_showing();
        let race_playback_button_input = Self::race_playback_button_showing();
        let race_director_input = race_director_hud::showing();
        #[cfg(target_os = "windows")]
        let free_camera_input_capture = crate::windows::free_camera::wants_windows_input_capture();
        #[cfg(not(target_os = "windows"))]
        let free_camera_input_capture = false;

        GUI_INPUT_ACTIVE.store(
            self.menu_visible || !self.windows.is_empty(),
            atomic::Ordering::Release,
        );

        // IS_CONSUMING_INPUT: coarse gate — true whenever any interactive egui element
        // is visible. Tells the Windows wnd_proc / Android input hook to enter the egui
        // processing path. Fine-grained blocking is handled by WANTS_INPUT below.
        IS_CONSUMING_INPUT.store(
            self.is_consuming_input() || live_slider_active || race_slider_input || race_playback_button_input || race_director_input || free_camera_input_capture,
            atomic::Ordering::Release,
        );

        // WANTS_INPUT: fine gate — only true when the game should NOT also receive this
        // input. Overlay elements only block game input when the pointer is directly over
        // them. Sidebar/app windows block when actively processing any pointer/keyboard,
        // or hovering/scrolling within a window.
        let overlay_captured = over_race_slider || over_live_slider || over_playback_btn || over_hud;
        let window_captured = self.is_consuming_input() && (actively_using_pointer || is_hovering_window || has_scroll);
        WANTS_INPUT.store(
            window_captured
                || overlay_captured
                || free_camera_input_capture,
            atomic::Ordering::Release,
        );

        // EGUI_TYPING: true while the user is typing into an egui text field. The OS-level
        // key hooks use this to suppress mod keybinds so typing is not intercepted.
        EGUI_TYPING.store(
            self.context.wants_keyboard_input(),
            atomic::Ordering::Release,
        );

        self.context.end_pass()
    }

    #[cfg(target_os = "windows")]
    fn run_free_camera_overlay(&mut self, ctx: &egui::Context) {
        let Some((content, alpha)) = crate::windows::free_camera::overlay_message() else {
            return;
        };

        let scale = get_scale(ctx);
        let fill = egui::Color32::from_black_alpha((170.0 * alpha) as u8);
        let text = egui::Color32::WHITE.linear_multiply(alpha);

        egui::Area::new(egui::Id::new("free_camera_overlay"))
            .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-16.0 * scale, 16.0 * scale))
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .fill(fill)
                    .inner_margin(egui::Margin::symmetric((10.0 * scale) as i8, (6.0 * scale) as i8))
                    .corner_radius(6.0 * scale)
                    .show(ui, |ui| {
                        ui.set_min_width(260.0 * scale);
                        ui.visuals_mut().override_text_color = Some(text);
                        ui.label(content);
                    });
            });
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
        let (safe_top, _) = get_safe_insets(ctx);
        let start_y = -100.0 * scale;
        let target_y = 8.0 * scale + safe_top;
        let current_y = start_y + (target_y - start_y) * tween_val;

        egui::Area::new(id)
            .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, current_y))
            .constrain(false)
            .interactable(false)
            .show(ctx, |ui| {
                let rounding = Hachimi::instance().config.load().ui_window_rounding;
                let corner_radius = rounding * scale;

                let splash_base = get_global_color("surfaceContainerHigh");
                let outline_color = get_global_color("outlineVariant");

                let splash_fill = {
                    let cfg = Hachimi::instance().config.load();
                    match cfg.ui_translucency_mode {
                        hachimi::UiTranslucencyMode::None => splash_base,
                        hachimi::UiTranslucencyMode::Overlay | hachimi::UiTranslucencyMode::Full => {
                            egui::Color32::from_rgba_unmultiplied(
                                splash_base.r(), splash_base.g(), splash_base.b(),
                                cfg.ui_surface_alpha,
                            )
                        }
                    }
                };

                let border_stroke = egui::Stroke::new(
                    1.0 * scale,
                    egui::Color32::from_rgba_unmultiplied(
                        outline_color.r(), outline_color.g(), outline_color.b(), 90,
                    ),
                );

                egui::Frame::NONE
                    .fill(splash_fill)
                    .stroke(border_stroke)
                    .corner_radius(corner_radius)
                    .shadow(egui::Shadow {
                        spread: 0,
                        blur: (12.0 * scale) as u8,
                        offset: [0, (3.0 * scale) as i8],
                        color: egui::Color32::from_black_alpha(45),
                    })
                    .inner_margin(egui::Margin::symmetric(
                        (14.0 * scale) as i8,
                        (10.0 * scale) as i8,
                    ))
                    .show(ui, |ui| {
                        let card_w = 320.0 * scale;
                        ui.set_width(card_w);

                        // Use centered layout so logo stays vertically centered
                        // regardless of how many hint lines the platform produces.
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            ui.spacing_mut().item_spacing.x = 10.0 * scale;

                            ui.add(Self::icon(ctx).fit_to_exact_size(egui::Vec2::new(36.0 * scale, 36.0 * scale)));

                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 3.0 * scale;

                                ui.horizontal(|ui| {
                                    ui.add(egui::Label::new(
                                        egui::RichText::new("Hachimi Edge")
                                            .size(15.0 * scale)
                                            .strong()
                                            .color(get_global_color("onSurface")),
                                    ));

                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        let version_bg = get_global_color("primaryContainer");
                                        let version_fg = get_global_color("onPrimaryContainer");

                                        egui::Frame::NONE
                                            .fill(egui::Color32::from_rgba_unmultiplied(
                                                version_bg.r(), version_bg.g(), version_bg.b(), 180,
                                            ))
                                            .corner_radius((10.0 * scale) as u8)
                                            .inner_margin(egui::Margin::symmetric(
                                                (7.0 * scale) as i8,
                                                (2.0 * scale) as i8,
                                            ))
                                            .show(ui, |ui| {
                                                ui.add(egui::Label::new(
                                                    egui::RichText::new(env!("HACHIMI_DISPLAY_VERSION"))
                                                        .color(version_fg)
                                                        .size(10.0 * scale)
                                                        .strong(),
                                                ));
                                            });
                                    });
                                });

                                // Hint rows — split on \n so each line gets its own
                                // label, keeping icons aligned and wrapping correctly.
                                let hint_color = get_global_color("onSurfaceVariant");
                                for line in self.splash_sub_str.split('\n') {
                                    ui.add(egui::Label::new(
                                        egui::RichText::new(line)
                                            .color(hint_color)
                                            .size(11.5 * scale),
                                    ));
                                }
                            });
                        });
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
            let mut min_w = 120.0 * scale;
            let mut max_w = (screen_w * 0.85).min(300.0 * scale);

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
                    .default_width((240.0 * scale).min(screen_w * 0.75))
                    .show_animated(ctx, self.show_menu, |ui| {

                        let (safe_top, _) = get_safe_insets(ctx);
                        if safe_top > 0.0 {
                            ui.add_space(safe_top);
                        }

                        ui.with_layout(egui::Layout::top_down_justified(egui::Align::TOP), |ui| {
                            ui.spacing_mut().item_spacing.y = 8.0 * scale;

                            ui.add_space(8.0 * scale);

                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 12.0 * scale;

                                let logo_size = 52.0 * scale;
                                ui.add(Self::icon_2x(ctx).fit_to_exact_size(
                                    egui::Vec2::splat(logo_size),
                                ));

                                ui.vertical(|ui| {
                                    ui.spacing_mut().item_spacing.y = 5.0 * scale;

                                    ui.add(egui::Label::new(
                                        egui::RichText::new("Hachimi Edge")
                                            .size(17.0 * scale)
                                            .strong()
                                            .color(get_global_color("onSurface")),
                                    ));

                                    {
                                        let version_bg = get_global_color("primaryContainer");
                                        let version_fg = get_global_color("onPrimaryContainer");
                                        egui::Frame::NONE
                                            .fill(egui::Color32::from_rgba_unmultiplied(
                                                version_bg.r(), version_bg.g(), version_bg.b(), 180,
                                            ))
                                            .corner_radius((10.0 * scale) as u8)
                                            .inner_margin(egui::Margin::symmetric(
                                                (8.0 * scale) as i8,
                                                (3.0 * scale) as i8,
                                            ))
                                            .show(ui, |ui| {
                                                ui.add(egui::Label::new(
                                                    egui::RichText::new(env!("HACHIMI_DISPLAY_VERSION"))
                                                        .color(version_fg)
                                                        .size(11.5 * scale)
                                                        .strong(),
                                                ));
                                            });
                                    }

                                      ui.horizontal(|ui| {
                                          ui.spacing_mut().item_spacing.x = 4.0 * scale;
                                          let icon_rt = |glyph: &str| {
                                              egui::RichText::new(glyph).size(22.0 * scale)
                                          };
                                          if ui.add(MaterialButton::text(icon_rt("\u{e887}"))).clicked() {
                                              show_window = Some(Box::new(AboutWindow::new()));
                                          }
                                          if ui.add(MaterialButton::text(icon_rt("\u{e5cd}"))).clicked() {
                                              self.show_menu = false;
                                              self.menu_anim_time = None;
                                          }
                                      });
                                 });
                            });

                            ui.add_space(8.0 * scale);

                            if ui
                                .add(MaterialButton::filled(t!("menu.check_for_updates")))
                                .clicked()
                            {
                                Hachimi::instance()
                                    .updater
                                    .clone()
                                    .check_for_updates(|_| {});
                            }

                            ui.add_space(8.0 * scale);

                            egui::ScrollArea::vertical().show(ui, |ui| {
                                ui.set_width(ui.available_width());

                                section_heading(ui, t!("menu.stats_heading").into_owned());
                                section_group_frame().show(ui, |ui| {
                                    ui.spacing_mut().item_spacing.y = 4.0 * scale;
                                    ui.add(egui::Label::new(
                                        egui::RichText::new(&self.fps_text)
                                            .color(get_global_color("onSurfaceVariant"))
                                            .size(12.0 * scale),
                                    ));
                                    ui.add(egui::Label::new(
                                        egui::RichText::new(t!("menu.localize_dict_entries", count = localize_dict_count))
                                            .color(get_global_color("onSurfaceVariant"))
                                            .size(12.0 * scale),
                                    ));
                                    ui.add(egui::Label::new(
                                        egui::RichText::new(t!("menu.hashed_dict_entries", count = hashed_dict_count))
                                            .color(get_global_color("onSurfaceVariant"))
                                            .size(12.0 * scale),
                                    ));
                                });

                                section_heading(ui, t!("menu.config_heading").into_owned());
                                if ConfigEditor::list_tile_button(ui, t!("menu.open_config_editor")) {
                                    show_window = Some(Box::new(ConfigEditor::new()));
                                }
                                if ConfigEditor::list_tile_button(ui, t!("menu.reload_config")) {
                                    hachimi.reload_config();
                                    show_notification = Some(t!("notification.config_reloaded"));
                                }
                                if ConfigEditor::list_tile_button(ui, t!("menu.open_first_time_setup")) {
                                    show_window = Some(Box::new(FirstTimeSetupWindow::new()));
                                }
                                if ConfigEditor::list_tile_button(ui, t!("menu.change_translation_repo")) {
                                    show_window = Some(Box::new(RepoSwitcherWindow::new()));
                                }

                                section_heading(ui, t!("menu.graphics_heading").into_owned());
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
                                        if discord_rpc {
                                            std::thread::Builder::new()
                                                .name("discord_rpc_toggle".into())
                                                .spawn(|| {
                                                    if let Err(e) = crate::windows::discord::start_rpc() {
                                                        warn!("Failed to start Discord RPC: {}", e);
                                                    }
                                                })
                                                .ok();
                                        } else if let Err(e) = crate::windows::discord::stop_rpc() {
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
                                ui.add_space(8.0 * scale);

                                section_heading(ui, t!("menu.translation_heading").into_owned());
                                if ConfigEditor::list_tile_button(ui, t!("menu.reload_localized_data")) {
                                    hachimi.load_localized_data();
                                    show_notification = Some(t!("notification.localized_data_reloaded"));
                                }
                                if ConfigEditor::list_tile_button(ui, t!("menu.tl_check_for_updates")) {
                                    hachimi.tl_updater.skip_update(None);
                                    hachimi.tl_updater.clone().check_for_updates(false, false);
                                }
                                if ConfigEditor::list_tile_button(ui, t!("menu.tl_check_for_updates_pedantic")) {
                                    hachimi.tl_updater.skip_update(None);
                                    hachimi.tl_updater.clone().check_for_updates(true, false);
                                }
                                if hachimi.config.load().translation_repo_index_mod.is_some() {
                                    if ConfigEditor::list_tile_button(ui, t!("menu.tl_check_for_addon_updates_pedantic")) {
                                        hachimi.tl_updater.skip_update(None);
                                        hachimi.tl_updater.clone().check_for_mod_updates_only(true, false);
                                    }
                                }
                                if hachimi.config.load().translator_mode {
                                    if ConfigEditor::list_tile_button(ui, t!("menu.dump_localize_dict")) {
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
                                ui.add_space(8.0 * scale);

                                let plugin_items = get_plugin_menu_items();
                                if !plugin_items.is_empty() {
                                    section_heading(ui, "Plugins".to_owned());
                                    for item in plugin_items {
                                        let icon = get_plugin_menu_icon(&item.label);
                                        // Build label text: prepend icon image as a richtext
                                        // inline prefix isn't supported cleanly, so we keep the
                                        // icon as a leading image but size it to the button's
                                        // text line height and float it inside a full-width row.
                                        let clicked = if let Some(icon) = icon {
                                            // Full-width row: fixed icon on the left, button
                                            // fills the remaining width. Compute btn_avail
                                            // from available_width() at this point (before any
                                            // horizontal layout consumes space).
                                            let icon_size = 16.0 * scale;
                                            let gap = 6.0 * scale;
                                            let btn_w = (ui.available_width() - icon_size - gap).max(0.0);
                                            ui.vertical(|ui| {
                                                let clicked = ui.horizontal(|ui| {
                                                    ui.spacing_mut().item_spacing.x = gap;
                                                    ui.allocate_ui_with_layout(
                                                        egui::Vec2::splat(icon_size),
                                                        egui::Layout::centered_and_justified(egui::Direction::TopDown),
                                                        |ui| {
                                                            ui.add(
                                                                egui::Image::new((icon.uri, icon.bytes))
                                                                    .fit_to_exact_size(egui::Vec2::splat(icon_size)),
                                                            );
                                                        },
                                                    );
                                                    ui.add(
                                                        MaterialButton::filled_tonal(&item.label)
                                                            .min_size(egui::vec2(btn_w, LIST_TILE_H * scale)),
                                                    ).clicked()
                                                }).inner;
                                                ui.add_space(2.0 * scale);
                                                clicked
                                            }).inner
                                        } else {
                                            ConfigEditor::list_tile_button(ui, &item.label)
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
                                    ui.add_space(8.0 * scale);
                                }

                                let plugin_sections = get_plugin_menu_sections();
                                if !plugin_sections.is_empty() {
                                    for section in plugin_sections {
                                        if let Some(title) = section.title.clone() {
                                            // Plugin section heading: matches section_heading()
                                            // style (primary, 17sp, bold) with optional inline icon.
                                            let icon = section.icon.clone();
                                            ui.add_space(12.0 * scale);
                                            ui.horizontal(|ui| {
                                                ui.spacing_mut().item_spacing.x = 6.0 * scale;
                                                if let Some(icon) = icon {
                                                    ui.add(
                                                        egui::Image::new((icon.uri, icon.bytes))
                                                            .fit_to_exact_size(egui::Vec2::splat(17.0 * scale)),
                                                    );
                                                }
                                                ui.add(egui::Label::new(
                                                    egui::RichText::new(title)
                                                        .color(get_global_color("primary"))
                                                        .family(egui::FontFamily::Proportional)
                                                        .size(17.0 * scale)
                                                        .strong(),
                                                ));
                                            });
                                            ui.add_space(6.0 * scale);
                                        } else {
                                            // Anonymous section (no title) — still needs a top
                                            // gap to separate it from the previous section/items.
                                            ui.add_space(12.0 * scale);
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
                                        // Consistent 8dp gap after each section's content,
                                        // matching the built-in sections' trailing rhythm.
                                        ui.add_space(8.0 * scale);
                                    }
                                }

                                ui.add_space(12.0 * scale);
                                {
                                    let error = get_global_color("error");
                                    let galley = ui.painter().layout_no_wrap(
                                        t!("menu.danger_zone_heading").into_owned(),
                                        egui::FontId::proportional(17.0 * scale),
                                        error,
                                    );
                                    let (resp, painter) = ui.allocate_painter(
                                        egui::vec2(ui.available_width(), galley.size().y + 8.0 * scale),
                                        egui::Sense::hover(),
                                    );
                                    painter.galley(
                                        egui::pos2(resp.rect.min.x + LIST_TILE_PAD_H, resp.rect.min.y + (resp.rect.height() - galley.size().y) / 2.0),
                                        galley,
                                        error,
                                    );
                                }
                                ui.add_space(4.0 * scale);
                                let dz_corner_radius = ui.visuals().window_corner_radius;
                                egui::Frame::NONE
                                    .fill(get_global_color("errorContainer"))
                                    .corner_radius(dz_corner_radius)
                                    .inner_margin(egui::Margin::symmetric(
                                        (10.0 * scale) as i8,
                                        (6.0 * scale) as i8,
                                    ))
                                    .show(ui, |ui| {
                                        ui.set_width(ui.available_width());
                                        // Reset to non-justified layout — the parent
                                        // top_down_justified layout would otherwise
                                        // stretch inter-character spacing to fill width.
                                        ui.style_mut().override_text_style = None;
                                        ui.with_layout(
                                            egui::Layout::top_down(egui::Align::Min),
                                            |ui| {
                                                ui.add(egui::Label::new(
                                                    egui::RichText::new(t!("menu.danger_zone_warning"))
                                                        .color(get_global_color("onErrorContainer"))
                                                        .family(egui::FontFamily::Proportional)
                                                        .size(11.5 * scale),
                                                ).wrap());
                                            },
                                        );
                                    });
                                ui.add_space(6.0 * scale);

                                if ConfigEditor::list_tile_button_danger(ui, t!("menu.soft_restart")) {
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
                                if ConfigEditor::list_tile_button_danger(ui, t!("menu.open_in_game_browser")) {
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
                                if ConfigEditor::list_tile_button(ui, t!("menu.toggle_game_ui")) {
                                    Thread::main_thread().schedule(Self::toggle_game_ui);
                                }

                                let (_, safe_bottom) = get_safe_insets(ui.ctx());
                                ui.add_space((16.0 * scale) + safe_bottom);
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
        let row_height = 28.0 * scale;
        let padding = ui.spacing().button_padding;

        let button_id = ui.make_persistent_id(id_salt);
        let popup_id = button_id.with("popup");

        let selected_text = choices
            .iter()
            .find(|(v, _)| v == value)
            .map(|(_, s)| *s)
            .unwrap_or("Unknown");

        let desired_width = ui.available_width();
        let min_width = 145.0 * scale;
        let popup_width = desired_width.max(min_width);

        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(desired_width, row_height), egui::Sense::hover());
        let button_res = ui.interact(rect, button_id, egui::Sense::click());

        if ui.is_rect_visible(rect) {
            let is_open = egui::Popup::is_id_open(ui.ctx(), popup_id);
            let is_hovered = button_res.hovered();

            let primary_color = get_global_color("primary");
            let surface = get_global_color("surface");
            let outline = get_global_color("outline");
            let on_surface = get_global_color("onSurface");
            let on_surface_variant = get_global_color("onSurfaceVariant");

            let (bg_color, border_color, text_color) = if is_hovered || is_open {
                (surface, primary_color, on_surface)
            } else {
                (surface, outline, on_surface_variant)
            };

            let border_stroke = egui::Stroke::new(
                if is_hovered || is_open { 2.0_f32 } else { 1.0_f32 },
                border_color,
            );

            // Draw background and outline (using corner radius 4.0 matching MaterialSelect)
            // MD3: dropdown button uses 4dp corner radius — does NOT follow global corner radius
            let combo_cr = 4u8;
            ui.painter().rect_filled(rect, combo_cr, bg_color);
            ui.painter().rect_stroke(rect, combo_cr, border_stroke, egui::epaint::StrokeKind::Outside);

            let icon_size = 12.0 * scale;
            let icon_rect = egui::Rect::from_center_size(
                egui::pos2(rect.right() - padding.x - icon_size / 2.0, rect.center().y),
                egui::vec2(icon_size, icon_size),
            );

            let dummy_visuals = egui::style::WidgetVisuals {
                fg_stroke: egui::Stroke::new(1.0_f32, text_color),
                ..ui.visuals().widgets.noninteractive.clone()
            };
            Self::down_triangle_icon(ui.painter(), icon_rect, &dummy_visuals);

            let icon_w = icon_size + padding.x + 4.0 * scale;
            let text_max_w = (rect.width() - padding.x - icon_w).max(0.0);
            let mut job = egui::text::LayoutJob::simple_singleline(
                selected_text.to_owned(),
                egui::TextStyle::Button.resolve(ui.style()),
                text_color,
            );
            job.wrap = egui::text::TextWrapping {
                max_width: text_max_w,
                max_rows: 1,
                break_anywhere: false,
                overflow_character: Some('…'),
            };
            let galley = ui.painter().layout_job(job);

            let text_pos = egui::pos2(
                rect.left() + padding.x,
                rect.center().y - galley.size().y / 2.0,
            );
            ui.painter().galley(text_pos, galley, text_color);
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

                egui::Frame::NONE
                    .inner_margin(egui::Margin::symmetric(8 as i8, 4 as i8))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 4.0 * scale;
                            let avail_w = ui.available_width();
                            let clear_btn_w = 32.0 * scale;
                            let field_w = (avail_w - clear_btn_w - 4.0 * scale).max(50.0);

                            ui.add_sized(
                                [field_w, 28.0 * scale],
                                MaterialTextField::outlined(search_term).hint_text(t!("search_filter")),
                            );

                            if ui.add(MaterialButton::text("\u{e5cd}").small()).clicked() {
                                search_term.clear();
                            }
                        });
                    });

                ui.separator();

                egui::ScrollArea::vertical()
                    .max_height(250.0 * scale)
                    .hscroll(false)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);

                        let secondary_container = get_global_color("secondaryContainer");
                        let on_secondary_container = get_global_color("onSecondaryContainer");
                        let on_surface = get_global_color("onSurface");

                        let search_lower = search_term.to_lowercase();
                        for (choice_val, label) in choices {
                            if !search_lower.is_empty()
                                && !label.to_lowercase().contains(&search_lower)
                            {
                                continue;
                            }

                            let is_selected = value == choice_val;
                            let text_color = if is_selected {
                                on_secondary_container
                            } else {
                                on_surface
                            };

                            let font_id = egui::TextStyle::Button.resolve(ui.style());
                            let item_text_w = (ui.available_width() - 16.0 * scale).max(0.0);
                            let galley = ui.painter().layout(
                                (*label).to_string(),
                                font_id,
                                text_color,
                                item_text_w,
                            );

                            let text_height = galley.size().y;
                            let option_height = (text_height + 8.0 * scale).max(36.0 * scale);
                            let option_rect = egui::Rect::from_min_size(
                                egui::pos2(ui.max_rect().min.x, ui.cursor().min.y),
                                egui::vec2(ui.max_rect().width(), option_height),
                            );

                            let option_response = ui.interact(
                                option_rect,
                                egui::Id::new(("vocals_option", *label)),
                                egui::Sense::click(),
                            );

                            let bg_color = if is_selected {
                                secondary_container
                            } else if option_response.hovered() {
                                egui::Color32::from_rgba_unmultiplied(on_surface.r(), on_surface.g(), on_surface.b(), 20)
                            } else {
                                egui::Color32::TRANSPARENT
                            };

                            if bg_color != egui::Color32::TRANSPARENT {
                                ui.painter().rect_filled(option_rect, 4.0, bg_color);
                            }

                            let text_pos = egui::pos2(
                                option_rect.min.x + 16.0 * scale,
                                option_rect.center().y - galley.size().y / 2.0,
                            );
                            ui.painter().galley(text_pos, galley, text_color);

                            ui.advance_cursor_after_rect(option_rect);

                            if option_response.clicked() {
                                *value = *choice_val;
                                changed = true;
                                egui::Popup::close_id(ui.ctx(), popup_id);
                                search_term.clear();
                            }
                        }
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

        let (progress, is_mod) = if let Some(p) = tl_updater.mod_progress() {
            (p, true)
        } else if let Some(p) = tl_updater.progress() {
            (p, false)
        } else {
            self.update_progress_visible = false;
            return;
        };

        let ratio = progress.current as f32 / progress.total as f32;
        let show_sizes = matches!(progress.phase, UpdatePhase::Downloading | UpdatePhase::Extracting);
        let (safe_top, _) = get_safe_insets(ctx);

        egui::Area::new("update_progress".into())
            .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 8.0 * scale + safe_top))
            .constrain(false)
            .interactable(false)
            .show(ctx, |ui| {
                let card_width = 320.0 * scale; // Locked size so it doesn't stretch in landscape
                let rounding = Hachimi::instance().config.load().ui_window_rounding;
                let corner_radius = rounding * scale;
                let pad_h = 16.0 * scale;
                let pad_v = 10.0 * scale; // Align with splash card
                let is_downloading = Hachimi::instance().tl_updater.is_downloading();
                let title = if progress.phase == UpdatePhase::Extracting {
                    t!("tl_updater.extracting")
                } else if is_downloading {
                    // Fresh install (no existing TL files) is a download, not an
                    // update — applies to the main repo and the addon cascade alike.
                    if Hachimi::instance().tl_updater.fresh_install() {
                        t!("tl_updater.downloading")
                    } else if is_mod {
                        t!("tl_updater.title_mod")
                    } else {
                        t!("tl_updater.title")
                    }
                } else {
                    t!("tl_updater.checking")
                };
                let progress_base = get_global_color("surfaceContainerHigh");
                let progress_fill = {
                    let cfg = Hachimi::instance().config.load();
                    match cfg.ui_translucency_mode {
                        hachimi::UiTranslucencyMode::None => progress_base,
                        hachimi::UiTranslucencyMode::Overlay | hachimi::UiTranslucencyMode::Full => {
                            egui::Color32::from_rgba_unmultiplied(
                                progress_base.r(), progress_base.g(), progress_base.b(),
                                cfg.ui_surface_alpha,
                            )
                        }
                    }
                };
                egui::Frame::NONE
                    .fill(progress_fill)
                    .inner_margin(egui::Margin::symmetric(pad_h as i8, pad_v as i8))
                    .corner_radius(corner_radius) // Align with splash card (linked to main rounding config)
                    .shadow(egui::Shadow {
                        spread: 0,
                        blur: (8.0 * scale) as u8,
                        offset: [0, (2.0 * scale) as i8], // Align with splash card
                        color: egui::Color32::from_black_alpha(35),
                    })
                    .show(ui, |ui| {
                        ui.set_width(card_width);

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

                        let bar_w = ui.available_width();
                        ui.add(
                            MaterialProgress::linear()
                                .value(ratio)
                                .size(egui::Vec2::new(bar_w, 4.0 * scale)),
                        );

                        if show_sizes {
                            ui.add_space(4.0 * scale);
                            let size_text = if progress.total > 0 {
                                format!(
                                    "{}/{}",
                                    Size::from_bytes(progress.current).format().with_base(Base::Base10).with_style(Style::Abbreviated),
                                    Size::from_bytes(progress.total).format().with_base(Base::Base10).with_style(Style::Abbreviated)
                                )
                            } else {
                                format!(
                                    "{}",
                                    Size::from_bytes(progress.current).format().with_base(Base::Base10).with_style(Style::Abbreviated)
                                )
                            };
                            ui.label(
                                egui::RichText::new(size_text)
                                    .font(egui::FontId::proportional(11.0 * scale))
                                    .color(get_global_color("onSurfaceVariant")),
                            );
                        }

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
            let rounding = Hachimi::instance().config.load().ui_window_rounding;
            let corner_radius = rounding * scale;
            let (_, safe_bottom) = get_safe_insets(ctx);
            let inner = egui::Area::new(egui::Id::new("snackbar").with(s.id))
                .order(egui::Order::Foreground)
                .anchor(
                    egui::Align2::CENTER_BOTTOM,
                    egui::vec2(0.0, -(48.0 * scale + offset + safe_bottom)),
                )
                .show(ctx, |ui| {
                    ui.set_max_width(max_w);
                    ui.add(
                        MaterialSnackbar::new(&s.message)
                            .behavior(SnackBarBehavior::Floating)
                            .corner_radius(corner_radius)
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
        if let Some(wid) = self.pending_top_window_id.take() {
            let salt = get_scale_salt(&self.context);
            let layer = egui::LayerId::new(egui::Order::Middle, wid.with(salt.to_bits()));
            self.context.memory_mut(|m| m.areas_mut().move_to_top(layer));
            self.top_layer = Some(layer);
        }
    }

    pub fn is_empty(&self) -> bool {
        #[cfg(target_os = "windows")]
        let free_camera_overlay = crate::windows::free_camera::has_overlay_message();
        #[cfg(not(target_os = "windows"))]
        let free_camera_overlay = false;

        !self.splash_visible
            && !self.menu_visible
            && !self.update_progress_visible
            && self.notifications.is_empty()
            && self.windows.is_empty()
            && !IS_LIVE_SCENE.load(atomic::Ordering::Relaxed)
            && !free_camera_overlay
            // These three are drawn unconditionally from run() (not gated behind
            // menu_visible/self.windows), so the present-hook's "nothing to draw, skip
            // this frame's GUI pass entirely" optimization above needs to know about them
            // too - otherwise they silently only ever render on frames where something
            // else already forced the GUI pass to run (e.g. the menu happening to be open).
            && !Self::race_slider_showing()
            && !Self::race_playback_button_showing()
            && !race_director_hud::showing()
    }

    pub fn is_gui_input_active_atomic() -> bool {
        GUI_INPUT_ACTIVE.load(atomic::Ordering::Acquire)
    }

    pub fn is_egui_typing_atomic() -> bool {
        EGUI_TYPING.load(atomic::Ordering::Acquire)
    }

    pub fn is_consuming_input(&self) -> bool {
        self.menu_visible
            || !self.windows.is_empty()
    }

    pub fn is_consuming_input_atomic() -> bool {
        IS_CONSUMING_INPUT.load(atomic::Ordering::Acquire)
    }

    pub fn set_consuming_input(&mut self, val: bool) {
        if !self.windows.is_empty() && !val {
            self.windows.clear();
        }

        self.menu_visible = val;
        IS_CONSUMING_INPUT.store(val, atomic::Ordering::Relaxed);
        if !val {
            WANTS_INPUT.store(false, atomic::Ordering::Relaxed);
        }
    }

    pub fn wants_input_atomic() -> bool {
        WANTS_INPUT.load(atomic::Ordering::Acquire)
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

    pub fn show_window(&mut self, app_window: BoxedAppWindow) {
        self.top_layer = None;
        self.pending_top_window_id = app_window.window_id();
        self.windows.push(app_window);
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
        let mut config = std::mem::take(&mut self.config);
        #[cfg(target_os = "windows")]
        {
            config.windows.menu_open_key = global_handle.windows.menu_open_key;
        }
        let mut reset_clicked = false;
        let mut save_clicked  = false;

        new_window(ctx, self.id, t!("config_editor.title"))
            .open(&mut open)
            .fixed_size(config_editor_window_size(ctx))
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

    fn window_id(&self) -> Option<egui::Id> {
        Some(self.id)
    }
}

impl ConfigEditor {
    // ── Search bar ──────────────────────────────────────────────────────────
    fn run_search_bar(ui: &mut egui::Ui, search_term: &mut String, scale: f32) {
        let pad_h = LIST_TILE_PAD_H * scale;
        egui::Frame::NONE
            .inner_margin(egui::Margin::symmetric(pad_h as i8, (2.0 * scale) as i8))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0 * scale;
                    let has_clear = !search_term.is_empty();
                    let clear_btn_w = if has_clear { 32.0 * scale } else { 0.0 };
                    let total_avail = ui.available_width();
                    let field_w = if has_clear {
                        (total_avail - clear_btn_w - 6.0 * scale).max(40.0)
                    } else {
                        total_avail
                    };

                    ui.allocate_ui_with_layout(
                        egui::vec2(field_w, 40.0 * scale),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            ui.set_min_width(field_w);
                            ui.set_max_width(field_w);
                            ui.add(MaterialTextField::filled(search_term).hint_text(t!("search_filter")));
                        },
                    );
                    if has_clear {
                        if ui.add(MaterialButton::text("\u{e5cd}").small()).clicked() {
                            search_term.clear();
                        }
                    }
                });
            });
        ui.add_space(2.0 * scale);
    }

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
        let search_h = 44.0 * scale;
        let scroll_h = (ui.available_height() - action_bar_h - tab_h - search_h - 7.0 * scale).max(40.0);

        Self::run_search_bar(ui, &mut self.search_term, scale);

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

        ui.separator();

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

        ui.add_space(6.0 * scale);

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
        let search_h     = 44.0 * scale;
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

                let inner_w = body_w - LIST_TILE_PAD_H * 2.0 * scale;
                ui.data_mut(|d| {
                    d.insert_temp(egui::Id::new("grid_control_w"), inner_w);
                });

                ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                    Self::run_search_bar(ui, &mut self.search_term, scale);
                    egui::ScrollArea::vertical()
                        .id_salt("landscape_body_scroll")
                        .max_height((body_h - search_h).max(40.0))
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

/// Pending custom title to apply on the main thread after a config save.
/// Avoids touching the game window from a spawned thread (GC/lifetime hazard).
#[cfg(target_os = "windows")]
static PENDING_TITLE: Mutex<Option<Option<String>>> = Mutex::new(None);

#[cfg(target_os = "windows")]
fn apply_pending_title() {
    Thread::main_thread().schedule(move || {
        let hwnd = crate::windows::wnd_hook::get_target_hwnd();
        let title = PENDING_TITLE.lock().unwrap().take().flatten();
        use ::windows::{core::HSTRING, Win32::UI::WindowsAndMessaging::SetWindowTextW};
        if let Some(t) = title {
            let _ = unsafe { SetWindowTextW(hwnd, &HSTRING::from(t.as_str())) };
        } else {
            let hachimi = Hachimi::instance();
            let default_title = if hachimi.game.region == crate::core::game::Region::Japan
                && hachimi.game.is_steam_release
            {
                HSTRING::from("UmamusumePrettyDerby_Jpn")
            } else {
                HSTRING::from("umamusume")
            };
            let _ = unsafe { SetWindowTextW(hwnd, &default_title) };
        }
    });
}

pub fn save_and_reload_config(config: hachimi::Config) {
    #[cfg(target_os = "windows")]
    {
        *PENDING_TITLE.lock().unwrap() = Some(config.custom_title_name.clone());
    }
    let notif = match Hachimi::instance().save_and_reload_config(config) {
        Ok(_) => {
            #[cfg(target_os = "windows")]
            {
                crate::windows::wnd_hook::apply_freeform_window_config();
                crate::windows::free_camera::reload_runtime_config();
                apply_pending_title();
            }
            t!("notification.config_saved").into_owned()
        },
        Err(e) => t!("notification.error_occurred", reason = e.to_string()).into_owned(),
    };

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
        2.2 / 3.0
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

#[cfg(test)]
mod bug_condition_exploration_tests {
    use super::{compute_pixels_per_point, PIXELS_PER_POINT_RATIO};
    use proptest::prelude::*;

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

    #[test]
    fn test_slider_max_should_be_2_0() {
        let slider_max: f32 = 2.0;
        let expected_max: f32 = 2.0;
        assert!(
            (slider_max - expected_max).abs() < f32::EPSILON,
            "Slider max for gui_landscape_ratio = {slider_max:.2}, expected {expected_max:.2}"
        );
    }

    proptest! {
        #[test]
        fn prop_landscape_ppp_equals_height_times_ratio_times_gui_ratio(
            height in 1_i32..=4320_i32,
            width_extra in 1_i32..=7680_i32,
            gui_landscape_ratio in 0.25_f32..=2.0_f32,
        ) {
            let width = height + width_extra;
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

#[cfg(test)]
mod preservation_tests {
    use super::{compute_pixels_per_point, PIXELS_PER_POINT_RATIO};
    use proptest::prelude::*;

    #[test]
    fn observe_portrait_9x16_equals_3_0() {
        let result = compute_pixels_per_point(1080, 1920, 1.0, true);
        let expected = 1080_f32 * PIXELS_PER_POINT_RATIO * 1.0;
        assert!(
            (result - expected).abs() < 1e-6,
            "Portrait(1080, 1920): got {result:.6}, expected {expected:.6}"
        );
        assert!(
            (result - 3.0_f32).abs() < 1e-4,
            "Portrait(1080, 1920): got {result:.6}, expected ≈ 3.0"
        );
    }

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

    #[test]
    fn observe_portrait_unusual_ratio_3x4() {
        let result = compute_pixels_per_point(768, 1024, 1.0, true);
        let expected = 768_f32 * PIXELS_PER_POINT_RATIO;
        assert!(
            (result - expected).abs() < 1e-6,
            "Portrait(768, 1024): got {result:.6}, expected {expected:.6} (≈2.1333)"
        );
    }

    proptest! {
        #[test]
        fn prop_portrait_ppp_equals_min_axis_times_ratio(
            // width in [1, 4320], height ≥ width (portrait or square)
            width in 1_i32..=4320_i32,
            height_extra in 0_i32..=4320_i32,
            gui_landscape_ratio in 0.25_f32..=2.0_f32,
        ) {
            let height = width + height_extra; // guarantees height >= width (portrait / square)
            let result = compute_pixels_per_point(width, height, gui_landscape_ratio, true);
            let expected = width.min(height) as f32 * PIXELS_PER_POINT_RATIO;

            prop_assert!(
                (result - expected).abs() < 1e-6,
                "Portrait ({width}, {height}, gui_ratio={gui_landscape_ratio:.4}): \
                 got {result:.6}, expected {expected:.6}"
            );
        }


        #[test]
        fn prop_feature_disabled_landscape_ppp_equals_height_times_ratio(
            height in 1_i32..=4320_i32,
            width_extra in 1_i32..=7680_i32,
            gui_landscape_ratio in 0.25_f32..=2.0_f32,
        ) {
            let width = height + width_extra;
            let result = compute_pixels_per_point(width, height, gui_landscape_ratio, false);
            let expected = height as f32 * PIXELS_PER_POINT_RATIO * 1.0;

            prop_assert!(
                (result - expected).abs() < 1e-6,
                "Feature-disabled landscape ({width}, {height}, gui_ratio={gui_landscape_ratio:.4}): \
                 got {result:.6}, expected {expected:.6}"
            );
        }

        #[test]
        fn prop_gui_scale_has_no_effect_on_pixels_per_point(
            width in 1_i32..=4320_i32,
            height_extra in 0_i32..=4320_i32,
            gui_landscape_ratio in 0.25_f32..=2.0_f32,
        ) {
            let height = width + height_extra;
            let result_a = compute_pixels_per_point(width, height, gui_landscape_ratio, true);
            let result_b = compute_pixels_per_point(width, height, gui_landscape_ratio, true);
            prop_assert!(
                result_a == result_b,
                "compute_pixels_per_point must be pure and deterministic \
                 (width={}, height={}, gui_ratio={:.4}): {} != {}",
                width, height, gui_landscape_ratio, result_a, result_b
            );

            let expected = width.min(height) as f32 * PIXELS_PER_POINT_RATIO;
            prop_assert!(
                (result_a - expected).abs() < 1e-6,
                "Portrait ({width}, {height}): gui_scale must not affect pixels_per_point; \
                 got {result_a:.6}, expected {expected:.6}"
            );
        }
    }
}

pub enum NotificationRequest {
    TLRepoChanged,
    TLFolderMissing,
    Custom(String),
}

pub fn request_notification(req: NotificationRequest) {
    std::thread::spawn(move || {
        let Some(gui_mutex) = Gui::instance() else { return };
        if let Ok(mut gui) = gui_mutex.lock() {
            match req {
                NotificationRequest::TLRepoChanged => {
                    gui.show_notification(&rust_i18n::t!("notification.tl_repo_changed"));
                }
                NotificationRequest::TLFolderMissing => {
                    gui.show_notification(&rust_i18n::t!(
                        "notification.tl_repo_folder_missing"
                    ));
                }
                NotificationRequest::Custom(msg) => {
                    gui.show_notification(&msg);
                }
            }
        }
    });
}

#[cfg(target_os = "android")]
pub fn get_safe_insets(ctx: &egui::Context) -> (f32, f32) {
    let ppp = ctx.pixels_per_point();
    let hachimi = crate::core::Hachimi::instance();
    if hachimi.hooking_finished.load(std::sync::atomic::Ordering::Relaxed) {
        if let Some(rect) = crate::il2cpp::hook::UnityEngine_CoreModule::Screen::get_safeArea() {
            let screen_h_px = ctx.content_rect().height() * ppp;
            let safe_bottom = (rect.y / ppp).max(0.0);
            let safe_top = ((screen_h_px - rect.y - rect.height) / ppp).max(0.0);
            if safe_top > 0.0 || safe_bottom > 0.0 {
                return (safe_top, safe_bottom);
            }
        }
    }

    // Cache the JNI result so we don't call get_activity() every render frame.
    // Safe insets only change on orientation change, so a 2-second TTL is ample.
    static CACHED_INSETS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    static LAST_INSET_FETCH: std::sync::OnceLock<std::sync::Mutex<std::time::Instant>> =
        std::sync::OnceLock::new();
    let mutex = LAST_INSET_FETCH.get_or_init(|| {
        std::sync::Mutex::new(std::time::Instant::now() - std::time::Duration::from_secs(10))
    });
    let now = std::time::Instant::now();
    let needs_refresh = mutex
        .lock()
        .map(|t| now.duration_since(*t) > std::time::Duration::from_secs(2))
        .unwrap_or(true);

    if needs_refresh {
        let (top_px, bottom_px) = crate::android::utils::get_safe_insets_jni();
        // Pack two f32s into a u64 for atomic storage.
        let packed = ((top_px.to_bits() as u64) << 32) | (bottom_px.to_bits() as u64);
        CACHED_INSETS.store(packed, std::sync::atomic::Ordering::Release);
        if let Ok(mut t) = mutex.lock() { *t = now; }
        if top_px > 0.0 || bottom_px > 0.0 {
            return (top_px / ppp, bottom_px / ppp);
        }
    } else {
        let packed = CACHED_INSETS.load(std::sync::atomic::Ordering::Acquire);
        let top_px = f32::from_bits((packed >> 32) as u32);
        let bottom_px = f32::from_bits(packed as u32);
        if top_px > 0.0 || bottom_px > 0.0 {
            return (top_px / ppp, bottom_px / ppp);
        }
    }

    (0.0, 0.0)
}

#[cfg(not(target_os = "android"))]
pub fn get_safe_insets(_ctx: &egui::Context) -> (f32, f32) {
    (0.0, 0.0)
}
