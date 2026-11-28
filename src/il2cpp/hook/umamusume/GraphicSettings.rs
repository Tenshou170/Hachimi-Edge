use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use serde::{Deserialize, Serialize};

use crate::{
    core::{gui::{dialogs::SimpleOkDialog, Gui}, Hachimi},
    il2cpp::{
        hook::UnityEngine_CoreModule::SystemInfo,
        symbols::{SingletonLike, get_field_from_name, get_method_addr},
        types::*
    }
};

use super::{LowResolutionCamera, SingleModeStartResultCharaViewer};

#[derive(Default, Copy, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[repr(i32)]
pub enum ModelShadowType {
    #[default]
    None = 0,
    Normal = 1
}

static mut CLASS: *mut Il2CppClass = 0 as _;
pub fn class() -> *mut Il2CppClass {
    unsafe { CLASS }
}

pub fn instance() -> *mut Il2CppObject {
    let Some(singleton) = SingletonLike::new(class()) else {
        return 0 as _;
    };
    singleton.instance()
}

def_field_value_accessors!(get__resolutionScale, set__resolutionScale, RESSCALE_FIELD, f32);
def_field_value_accessors!(get__resolutionScale2D, set__resolutionScale2D, RESSCALE2D_FIELD, f32);

static ORIG_RES_SCALE: AtomicU32 = AtomicU32::new(u32::MAX);
static ORIG_RES_SCALE_2D: AtomicU32 = AtomicU32::new(u32::MAX);

fn sync_render_scale_field(
    this: *mut Il2CppObject,
    field: *mut crate::il2cpp::types::FieldInfo,
    orig_store: &AtomicU32,
    get_fn: fn(*mut Il2CppObject) -> f32,
    set_fn: fn(*mut Il2CppObject, f32),
    bypass: bool,
) {
    if field.is_null() {
        return;
    }
    let render_scale = if bypass {
        1.0
    } else {
        Hachimi::instance().config.load().render_scale.clamp(0.1, 4.0)
    };
    if render_scale != 1.0 {
        let cur = get_fn(this);
        if orig_store.load(Ordering::Relaxed) == u32::MAX {
            orig_store.store(cur.to_bits(), Ordering::Relaxed);
        }
        if cur != render_scale {
            set_fn(this, render_scale);
        }
    } else {
        let orig_bits = orig_store.swap(u32::MAX, Ordering::Relaxed);
        if orig_bits != u32::MAX {
            set_fn(this, f32::from_bits(orig_bits));
        }
    }
}

type GetVirtualResolutionFn = extern "C" fn(this: *mut Il2CppObject) -> Vector2Int_t;
extern "C" fn GetVirtualResolution(this: *mut Il2CppObject) -> Vector2Int_t {
    let bypass = !super::GameSystem::is_initialized();
    unsafe {
        sync_render_scale_field(this, RESSCALE2D_FIELD, &ORIG_RES_SCALE_2D, get__resolutionScale2D, set__resolutionScale2D, bypass);
    }
    let mut res = get_orig_fn!(GetVirtualResolution, GetVirtualResolutionFn)(this);
    let mult = Hachimi::instance().config.load().virtual_res_mult;
    if !bypass && mult != 1.0 {
        res *= mult;
    }
    res
}

static VRAM_GUARD_ACTIVE: AtomicBool = AtomicBool::new(false);

fn exceeds_gpu_limits(res: Vector2Int_t, msaa: MsaaQuality) -> Option<(i32, i32, u64, u64, i32)> {
    let msaa_samples = match msaa {
        MsaaQuality::Disabled => 1_u64,
        m => m as u64,
    };
    let pixels = (res.x.max(0) as u64) * (res.y.max(0) as u64);
    let est_mb = (pixels * (12 * msaa_samples + 24)) / (1024 * 1024);

    let max_tex = SystemInfo::max_texture_size();
    let max_tex = if max_tex > 0 { max_tex } else { 16384 };

    let vram_mb = SystemInfo::graphics_memory_size_mb();
    let vram_mb = if vram_mb > 0 { vram_mb } else { 4096 };
    let budget_mb = (vram_mb as u64) * 40 / 100;

    if res.x > max_tex || res.y > max_tex || est_mb > budget_mb {
        Some((max_tex, msaa_samples as i32, est_mb, budget_mb, vram_mb))
    } else {
        None
    }
}

fn trigger_vram_guard_reset(res: Vector2Int_t, max_tex: i32, msaa_samples: i32, est_mb: u64, budget_mb: u64, vram_mb: i32) {
    if VRAM_GUARD_ACTIVE.compare_exchange(false, true, Ordering::SeqCst, Ordering::Relaxed).is_err() {
        return;
    }
    warn!(
        "Resolution {}x{} ({}x MSAA, ~{} MB est. RT VRAM) exceeds GPU limits (max {} px, {}/{} MB VRAM); resetting render_scale and virtual_res_mult to 1.0",
        res.x, res.y, msaa_samples, est_mb, max_tex, budget_mb, vram_mb
    );
    std::thread::spawn(move || {
        let hachimi = Hachimi::instance();
        let mut cfg = (*hachimi.config.load_full()).clone();
        cfg.render_scale = 1.0;
        cfg.virtual_res_mult = 1.0;
        if let Err(e) = hachimi.save_and_reload_config(cfg) {
            error!("Failed to save config after VRAM guard reset: {}", e);
        }
        if let Some(gui_mutex) = Gui::instance() {
            let msg = format!(
                "Render Scale and Virtual Resolution Multiplier were reset to 1.0 because {}x{} ({}x MSAA, ~{} MB est. RT VRAM) exceeded safe GPU limits (max {} px, {} MB / {} MB VRAM).",
                res.x, res.y, msaa_samples, est_mb, max_tex, budget_mb, vram_mb
            );
            gui_mutex.lock().unwrap_or_else(|e| e.into_inner())
                .show_window(Box::new(SimpleOkDialog::new("Graphics Guard", &msg, || {})));
        }
        VRAM_GUARD_ACTIVE.store(false, Ordering::Release);
    });
}

type GetVirtualResolution3DFn = extern "C" fn(this: *mut Il2CppObject, is_forced_wide_aspect: bool) -> Vector2Int_t;
extern "C" fn GetVirtualResolution3D(this: *mut Il2CppObject, is_forced_wide_aspect: bool) -> Vector2Int_t {
    let bypass = !super::GameSystem::is_initialized()
        || SingleModeStartResultCharaViewer::setting_up_image_effect()
        || LowResolutionCamera::creating_render_texture();
    unsafe {
        sync_render_scale_field(this, RESSCALE_FIELD, &ORIG_RES_SCALE, get__resolutionScale, set__resolutionScale, bypass);
    }
    let orig_fn = get_orig_fn!(GetVirtualResolution3D, GetVirtualResolution3DFn);
    let mut res = orig_fn(this, is_forced_wide_aspect);
    let config = Hachimi::instance().config.load();
    let mult = config.virtual_res_mult;
    if !bypass {
        if mult != 1.0 {
            res *= mult;
        }
        if config.render_scale != 1.0 || mult != 1.0 {
            if let Some((max_tex, msaa_samples, est_mb, budget_mb, vram_mb)) = exceeds_gpu_limits(res, config.msaa) {
                unsafe {
                    sync_render_scale_field(this, RESSCALE_FIELD, &ORIG_RES_SCALE, get__resolutionScale, set__resolutionScale, true);
                }
                trigger_vram_guard_reset(res, max_tex, msaa_samples, est_mb, budget_mb, vram_mb);
                return orig_fn(this, is_forced_wide_aspect);
            }
        }
    }
    res
}

type GetVirtualResolutionWidth3DFn = extern "C" fn(this: *mut Il2CppObject) -> i32;
extern "C" fn GetVirtualResolutionWidth3D(this: *mut Il2CppObject) -> i32 {
    let mut width = get_orig_fn!(GetVirtualResolutionWidth3D, GetVirtualResolutionWidth3DFn)(this);
    let mult = Hachimi::instance().config.load().virtual_res_mult;
    if mult != 1.0 {
        width = (width as f32 * mult) as i32;
    }
    width
}

#[cfg(target_os = "windows")]
static mut UPDATE3DRENDERTEXTURE_ADDR: usize = 0;
#[cfg(target_os = "windows")]
impl_addr_wrapper_fn!(Update3DRenderTexture, UPDATE3DRENDERTEXTURE_ADDR, (), this: *mut Il2CppObject);

#[derive(Default, Copy, Clone, Serialize, Deserialize, Eq, PartialEq, Debug, Hash)]
#[repr(i32)]
pub enum GraphicsQuality {
    #[default] Default = -1,
    Toon1280 = 0,
    Toon1280x2,
    Toon1280x4,
    ToonFull,
    Max
}

// UnityEngine.Rendering.Universal 
#[derive(Default, Copy, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[repr(i32)]
pub enum MsaaQuality {
    #[default] Disabled = 1,
    _2x = 2,
    _4x = 4,
    _8x = 8
}

def_field_value_accessors!(set set__isMSAA, ISMSAA_FIELD, bool);

type get_IsMSAAFn = extern "C" fn(this: *mut Il2CppObject) -> bool;
pub extern "C" fn get_IsMSAA(this: *mut Il2CppObject) -> bool {
    if Hachimi::instance().config.load().msaa != MsaaQuality::Disabled {
        set__isMSAA(this, true);
        return true;
    }
    get_orig_fn!(get_IsMSAA, get_IsMSAAFn)(this)
}

type set_ResolutionScaleFn = extern "C" fn(this: *mut Il2CppObject, value: f32);
extern "C" fn set_ResolutionScale(this: *mut Il2CppObject, value: f32) {
    let render_scale = Hachimi::instance().config.load().render_scale;
    let target_value = if render_scale != 1.0 { render_scale } else { value };
    crate::core::utils::size_trace::event_msg("set_ResolutionScale", &format!(
        "requested={:.4} -> {:.4}", value, target_value
    ));
    get_orig_fn!(set_ResolutionScale, set_ResolutionScaleFn)(this, target_value);
}

type set_ResolutionScale2DFn = extern "C" fn(this: *mut Il2CppObject, value: f32);
pub extern "C" fn set_ResolutionScale2D(this: *mut Il2CppObject, value: f32) {
    let render_scale = Hachimi::instance().config.load().render_scale;
    let target_value = if render_scale != 1.0 { render_scale } else { value };
    crate::core::utils::size_trace::event_msg("set_ResolutionScale2D", &format!(
        "requested={:.4} -> {:.4}", value, target_value
    ));
    get_orig_fn!(set_ResolutionScale2D, set_ResolutionScale2DFn)(this, target_value);
}

type Get3DAntiAliasingLevelFn = extern "C" fn(this: *mut Il2CppObject, allowMSAA: bool) -> i32;
extern "C" fn Get3DAntiAliasingLevel(this: *mut Il2CppObject, allowMSAA: bool) -> i32 {
    let msaa = Hachimi::instance().config.load().msaa;
    if allowMSAA && msaa != MsaaQuality::Disabled {
        if unsafe { !ISMSAA_FIELD.is_null() } {
            set__isMSAA(this, true);
        }
        return msaa as i32;
    }
    get_orig_fn!(Get3DAntiAliasingLevel, Get3DAntiAliasingLevelFn)(this, allowMSAA)
}

type ApplyGraphicsQualityFn = extern "C" fn(this: *mut Il2CppObject, quality: GraphicsQuality, force: bool);
extern "C" fn ApplyGraphicsQuality(this: *mut Il2CppObject, quality: GraphicsQuality, force: bool) {
    let custom_quality = Hachimi::instance().config.load().graphics_quality;
    if custom_quality != GraphicsQuality::Default {
        crate::core::utils::size_trace::event_msg("ApplyGraphicsQuality", &format!(
            "requested={:?} force={} -> applied={:?} force=true",
            quality, force, custom_quality
        ));
        return get_orig_fn!(ApplyGraphicsQuality, ApplyGraphicsQualityFn)(this, custom_quality, true);
    }

    crate::core::utils::size_trace::event_msg("ApplyGraphicsQuality", &format!(
        "requested={:?} force={} (passthrough)", quality, force
    ));
    get_orig_fn!(ApplyGraphicsQuality, ApplyGraphicsQualityFn)(this, quality, force);
}

type GetModelShadowTypeFn = extern "C" fn(this: *mut Il2CppObject) -> ModelShadowType;
extern "C" fn GetModelShadowType(this: *mut Il2CppObject) -> ModelShadowType {
    if Hachimi::instance().config.load().force_chara_shadows {
        return ModelShadowType::Normal;
    }
    get_orig_fn!(GetModelShadowType, GetModelShadowTypeFn)(this)
}

pub fn init(umamusume: *const Il2CppImage) {
    get_class_or_return!(umamusume, Gallop, GraphicSettings);

    let GetVirtualResolution3D_addr = get_method_addr(GraphicSettings, c"GetVirtualResolution3D", 1);
    let GetVirtualResolution_addr = get_method_addr(GraphicSettings, c"GetVirtualResolution", 0);
    let GetVirtualResolutionWidth3D_addr = get_method_addr(GraphicSettings, c"GetVirtualResolutionWidth3D", 0);
    let ApplyGraphicsQuality_addr = get_method_addr(GraphicSettings, c"ApplyGraphicsQuality", 2);

    let get_IsMSAA_addr = get_method_addr(GraphicSettings, c"get_IsMSAA", 0);
    let SetResolutionScale_addr = get_method_addr(GraphicSettings, c"set_ResolutionScale", 1);
    let SetResolutionScale2D_addr = get_method_addr(GraphicSettings, c"set_ResolutionScale2D", 1);
    let Get3DAntiAliasingLevel_addr = get_method_addr(GraphicSettings, c"Get3DAntiAliasingLevel", 1);

    new_hook!(GetVirtualResolution3D_addr, GetVirtualResolution3D);
    new_hook!(GetVirtualResolution_addr, GetVirtualResolution);
    new_hook!(GetVirtualResolutionWidth3D_addr, GetVirtualResolutionWidth3D);
    new_hook!(ApplyGraphicsQuality_addr, ApplyGraphicsQuality);

    new_hook!(get_IsMSAA_addr, get_IsMSAA);
    new_hook!(SetResolutionScale_addr, set_ResolutionScale);
    new_hook!(SetResolutionScale2D_addr, set_ResolutionScale2D);
    new_hook!(Get3DAntiAliasingLevel_addr, Get3DAntiAliasingLevel);

    let GetModelShadowType_addr = get_method_addr(GraphicSettings, c"GetModelShadowType", 0);
    if GetModelShadowType_addr != 0 {
        new_hook!(GetModelShadowType_addr, GetModelShadowType);
    }

    unsafe {
        CLASS = GraphicSettings;
        ISMSAA_FIELD = get_field_from_name(GraphicSettings, c"_isMSAA");
        RESSCALE_FIELD = get_field_from_name(GraphicSettings, c"_resolutionScale");
        RESSCALE2D_FIELD = get_field_from_name(GraphicSettings, c"_resolutionScale2D");
    }

    #[cfg(target_os = "windows")]
    unsafe {
        UPDATE3DRENDERTEXTURE_ADDR = get_method_addr(GraphicSettings, c"Update3DRenderTexture", 0);
    }
}