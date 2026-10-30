//! Runtime feature availability, probed from the game's IL2CPP surface.
//!
//! The mod ships against two clients (Steam JP and Steam Global) whose
//! assemblies are forked: a class, method or field that exists on one client is
//! frequently absent from the other, and the shapes drift independently in both
//! directions (Global is sometimes *ahead* of JP, sometimes behind).
//!
//! Region checks are therefore only a hint. A capability is set by the hook
//! module that owns the feature, *after* every symbol it needs has been
//! resolved. The config UI reads these flags, so a control whose backing game
//! API is missing is hidden instead of silently doing nothing. Because the
//! flags are set from probes rather than from `Region`, a client that gains a
//! feature in a future patch lights the control up without a code change.

use std::sync::atomic::{AtomicU32, Ordering};

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Capability {
    /// `Gallop.RenderPipeline` shadow stack: `CameraData`, `CascadeShadow`,
    /// `CascadeShadowForRace` and `GallopRenderer` all present. Backs the
    /// shadow resolution / distance / soft shadow / bias / character shadow
    /// options. Absent from the Global client entirely.
    UrpShadowPipeline = 1 << 0,
    /// `StoryTimelineBg3DClipData.ShadowType` field, used by the story 3D
    /// shadow override.
    StoryShadowType = 1 << 1,
    /// Free-form window support: `Screen.SetResolution` plus
    /// `get_IsLandscapeMode`. Disabled on clients that drive their own window.
    FreeformWindow = 1 << 2,
    /// `ApplicationSettingSaveLoader.get_IsTryRaceDynamicCamera`.
    RaceDynamicCameraSetting = 1 << 3,
    /// `RaceUtil.GetRaceDynamicCameraSettingData`.
    RaceDynamicCameraUtil = 1 << 4,
    /// Trainer live FOV override: `Director.ApplyTrainerCameraFov` and the
    /// `_trainerCameraTargetCamera` field it drives.
    TrainerLiveFov = 1 << 5,
}

static AVAILABLE: AtomicU32 = AtomicU32::new(0);

/// Records that every symbol backing `capability` resolved on this client.
pub fn set(capability: Capability) {
    AVAILABLE.fetch_or(capability as u32, Ordering::Relaxed);
}

pub fn has(capability: Capability) -> bool {
    AVAILABLE.load(Ordering::Relaxed) & (capability as u32) != 0
}

/// Both halves of the "allow dynamic camera" toggle must be present: the
/// setting loader exposes the game's own preference, the util builds the
/// dynamic camera settings from it.
pub fn race_dynamic_camera() -> bool {
    has(Capability::RaceDynamicCameraSetting) && has(Capability::RaceDynamicCameraUtil)
}

/// Comma-separated list of unavailable capabilities, for the startup report.
pub fn unavailable_list() -> String {
    const ALL: &[(Capability, &str)] = &[
        (Capability::UrpShadowPipeline, "urp_shadow_pipeline"),
        (Capability::StoryShadowType, "story_shadow_type"),
        (Capability::FreeformWindow, "freeform_window"),
        (Capability::RaceDynamicCameraSetting, "race_dynamic_camera_setting"),
        (Capability::RaceDynamicCameraUtil, "race_dynamic_camera_util"),
        (Capability::TrainerLiveFov, "trainer_live_fov"),
    ];

    ALL.iter()
        .filter(|(cap, _)| !has(*cap))
        .map(|(_, name)| *name)
        .collect::<Vec<_>>()
        .join(", ")
}
