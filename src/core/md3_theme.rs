//! MD3 tonal palette generation and theme loading for Hachimi-Edge.
//!
//! Converts a seed `egui::Color32` into a full Material Design 3 tonal palette,
//! serialises it to the MaterialThemeFile JSON format, and loads it into the
//! egui-material3 global theme context.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use egui::Color32;
use log::error;

use egui_material3::theme::{
    apply_theme, get_global_theme, load_theme_from_json_str, set_global_corner_radius,
    ContrastLevel, ThemeMode,
};

use material_colors::{
    color::Argb,
    dynamic_color::{dynamic_scheme::DynamicScheme, variant::Variant},
    scheme::Scheme,
};

use crate::core::hachimi::{UiColorSchemeMode, UiContrastLevel, UiThemeMode};

// Cache key — re-apply whenever any theme-affecting field changes

#[derive(Clone, Copy, PartialEq)]
struct CacheKey {
    seed: Color32,
    mode: UiThemeMode,
    contrast: UiContrastLevel,
    scheme_mode: UiColorSchemeMode,
    surface_alpha: u8,
    rounding_bits: u32,
}

static CURRENT_KEY: Mutex<Option<CacheKey>> = Mutex::new(None);

/// Returns the seed color that was last successfully applied, or `None`.
pub fn current_seed() -> Option<Color32> {
    CURRENT_KEY.lock().ok().and_then(|g| g.map(|k| k.seed))
}

// Public entry point

/// Parameters bundled from `hachimi::Config` for theme application.
pub struct ThemeParams<'a> {
    pub seed: Color32,
    pub cached_json: Option<&'a str>,
    pub theme_mode: UiThemeMode,
    pub contrast_level: UiContrastLevel,
    pub scheme_mode: UiColorSchemeMode,
    /// Manual per-role overrides (used when scheme_mode == Manual).
    pub manual_colors: &'a HashMap<String, [u8; 3]>,
    /// Alpha (0–255) applied to surface-tier colors for transparency.
    pub surface_alpha: u8,
    /// Window/widget corner radius in logical pixels.
    pub window_rounding: f32,
}

/// Apply the theme to `ctx` from the given parameters.
///
/// Returns the generated theme JSON string if a new one was produced
/// (i.e. the seed changed or no cached JSON was available), so the
/// caller can persist it back into `Config::ui_theme_json`.
///
/// Returns `None` if the theme was already up-to-date (cache hit on
/// the same seed + mode + contrast combination).
pub fn apply_seed(
    ctx: &egui::Context,
    params: ThemeParams<'_>,
    _data_dir: &Path,
) -> Option<String> {
    // Resolved egui-material3 ThemeMode
    let egui_mode = match params.theme_mode {
        UiThemeMode::Dark => ThemeMode::Dark,
        UiThemeMode::Light => ThemeMode::Light,
    };
    let egui_contrast = match params.contrast_level {
        UiContrastLevel::Normal => ContrastLevel::Normal,
        UiContrastLevel::Medium => ContrastLevel::Medium,
        UiContrastLevel::High => ContrastLevel::High,
    };

    // Cache key: seed + mode + contrast.  Any change re-applies.
    let cache_key = CacheKey {
        seed: params.seed,
        mode: params.theme_mode,
        contrast: params.contrast_level,
        scheme_mode: params.scheme_mode,
        surface_alpha: params.surface_alpha,
        rounding_bits: params.window_rounding.to_bits(),
    };
    if let Ok(guard) = CURRENT_KEY.lock() {
        if *guard == Some(cache_key) {
            return None;
        }
    }

    let mut produced_json: Option<String> = None;

    match params.scheme_mode {
        UiColorSchemeMode::Auto => {
            // Use cached JSON when available, otherwise generate from seed
            let json = if let Some(j) = params.cached_json.filter(|s| !s.is_empty()) {
                j.to_owned()
            } else {
                generate_theme_json(params.seed)
            };

            match load_theme_from_json_str(&json) {
                Ok(()) => {
                    produced_json = Some(json);
                }
                Err(e) => {
                    error!("md3_theme: load failed ({}), regenerating", e);
                    let fresh = generate_theme_json(params.seed);
                    match load_theme_from_json_str(&fresh) {
                        Ok(()) => {
                            produced_json = Some(fresh);
                        }
                        Err(e2) => {
                            error!("md3_theme: regeneration also failed: {}", e2);
                        }
                    }
                }
            }
        }
        UiColorSchemeMode::Manual => {
            // In manual mode we start from the auto palette as a base and
            // then override individual roles via selected_colors below.
            // This means the user only needs to specify the roles they want
            // to change; everything else falls back to the HCT-derived values.
            let json = if let Some(j) = params.cached_json.filter(|s| !s.is_empty()) {
                j.to_owned()
            } else {
                generate_theme_json(params.seed)
            };
            match load_theme_from_json_str(&json) {
                Ok(()) => {
                    produced_json = Some(json);
                }
                Err(e) => {
                    error!("md3_theme: manual base load failed: {}", e);
                }
            }
        }
    }

    if let Ok(mut theme) = get_global_theme().lock() {
        theme.theme_mode = egui_mode;
        theme.contrast_level = egui_contrast;
        theme.selected_colors.clear();

        if params.scheme_mode == UiColorSchemeMode::Manual {
            for (role, rgb) in params.manual_colors {
                theme
                    .selected_colors
                    .insert(role.clone(), Color32::from_rgb(rgb[0], rgb[1], rgb[2]));
            }
        }

        // Apply transparency to surface-tier roles
        if params.surface_alpha < 255 {
            let alpha = params.surface_alpha;
            let surface_roles = [
                "surface",
                "surfaceContainer",
                "surfaceContainerHigh",
                "surfaceContainerHighest",
                "surfaceContainerLow",
                "surfaceContainerLowest",
                "surfaceDim",
                "surfaceBright",
                "background",
            ];
            for role in &surface_roles {
                // Only override if not already manually set
                if params.scheme_mode == UiColorSchemeMode::Manual
                    && params.manual_colors.contains_key(*role)
                {
                    continue;
                }
                let base = theme.get_color_by_name(role);
                theme.selected_colors.insert(
                    role.to_string(),
                    Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), alpha),
                );
            }
        }
    }

    let mode_clone = egui_mode;
    // Set global corner radius for MD3 widgets BEFORE apply_theme so buttons/dialogs
    // pick it up during their own rendering. None = use each widget's MD3 spec default.
    set_global_corner_radius(Some(params.window_rounding));
    apply_theme(ctx, Some(move || mode_clone));

    {
        let r = params.window_rounding;
        ctx.style_mut(|style| {
            let cr = egui::CornerRadius::same(r.round() as u8);
            style.visuals.window_corner_radius = cr;
            style.visuals.widgets.noninteractive.corner_radius = cr;
            style.visuals.widgets.inactive.corner_radius = cr;
            style.visuals.widgets.hovered.corner_radius = cr;
            style.visuals.widgets.active.corner_radius = cr;
            style.visuals.widgets.open.corner_radius = cr;
        });
    }

    if let Ok(mut guard) = CURRENT_KEY.lock() {
        *guard = Some(cache_key);
    }

    produced_json
}

// Material Design 3 Helper Functions

fn color32_to_argb(c: Color32) -> Argb {
    Argb::new(c.a(), c.r(), c.g(), c.b())
}

fn argb_to_hex(argb: Argb) -> String {
    argb.to_hex_with_pound()
}

fn scheme_to_map(scheme: Scheme) -> serde_json::Map<String, serde_json::Value> {
    let mut map = serde_json::Map::new();
    let insert = |m: &mut serde_json::Map<String, serde_json::Value>, k: &str, v: Argb| {
        m.insert(
            k.to_string(),
            serde_json::Value::String(v.to_hex_with_pound()),
        );
    };

    insert(&mut map, "primary", scheme.primary);
    insert(&mut map, "onPrimary", scheme.on_primary);
    insert(&mut map, "primaryContainer", scheme.primary_container);
    insert(&mut map, "onPrimaryContainer", scheme.on_primary_container);
    insert(&mut map, "inversePrimary", scheme.inverse_primary);
    insert(&mut map, "primaryFixed", scheme.primary_fixed);
    insert(&mut map, "primaryFixedDim", scheme.primary_fixed_dim);
    insert(&mut map, "onPrimaryFixed", scheme.on_primary_fixed);
    insert(
        &mut map,
        "onPrimaryFixedVariant",
        scheme.on_primary_fixed_variant,
    );
    insert(&mut map, "secondary", scheme.secondary);
    insert(&mut map, "onSecondary", scheme.on_secondary);
    insert(&mut map, "secondaryContainer", scheme.secondary_container);
    insert(
        &mut map,
        "onSecondaryContainer",
        scheme.on_secondary_container,
    );
    insert(&mut map, "secondaryFixed", scheme.secondary_fixed);
    insert(&mut map, "secondaryFixedDim", scheme.secondary_fixed_dim);
    insert(&mut map, "onSecondaryFixed", scheme.on_secondary_fixed);
    insert(
        &mut map,
        "onSecondaryFixedVariant",
        scheme.on_secondary_fixed_variant,
    );
    insert(&mut map, "tertiary", scheme.tertiary);
    insert(&mut map, "onTertiary", scheme.on_tertiary);
    insert(&mut map, "tertiaryContainer", scheme.tertiary_container);
    insert(
        &mut map,
        "onTertiaryContainer",
        scheme.on_tertiary_container,
    );
    insert(&mut map, "tertiaryFixed", scheme.tertiary_fixed);
    insert(&mut map, "tertiaryFixedDim", scheme.tertiary_fixed_dim);
    insert(&mut map, "onTertiaryFixed", scheme.on_tertiary_fixed);
    insert(
        &mut map,
        "onTertiaryFixedVariant",
        scheme.on_tertiary_fixed_variant,
    );
    insert(&mut map, "error", scheme.error);
    insert(&mut map, "onError", scheme.on_error);
    insert(&mut map, "errorContainer", scheme.error_container);
    insert(&mut map, "onErrorContainer", scheme.on_error_container);
    insert(&mut map, "surfaceDim", scheme.surface_dim);
    insert(&mut map, "surface", scheme.surface);
    insert(&mut map, "surfaceTint", scheme.surface_tint);
    insert(&mut map, "surfaceBright", scheme.surface_bright);
    insert(
        &mut map,
        "surfaceContainerLowest",
        scheme.surface_container_lowest,
    );
    insert(
        &mut map,
        "surfaceContainerLow",
        scheme.surface_container_low,
    );
    insert(&mut map, "surfaceContainer", scheme.surface_container);
    insert(
        &mut map,
        "surfaceContainerHigh",
        scheme.surface_container_high,
    );
    insert(
        &mut map,
        "surfaceContainerHighest",
        scheme.surface_container_highest,
    );
    insert(&mut map, "onSurface", scheme.on_surface);
    insert(&mut map, "onSurfaceVariant", scheme.on_surface_variant);
    insert(&mut map, "outline", scheme.outline);
    insert(&mut map, "outlineVariant", scheme.outline_variant);
    insert(&mut map, "inverseSurface", scheme.inverse_surface);
    insert(&mut map, "inverseOnSurface", scheme.inverse_on_surface);
    insert(&mut map, "surfaceVariant", scheme.surface_variant);
    insert(&mut map, "background", scheme.background);
    insert(&mut map, "onBackground", scheme.on_background);
    insert(&mut map, "shadow", scheme.shadow);
    insert(&mut map, "scrim", scheme.scrim);

    map
}

/// Build a MaterialThemeFile-format JSON string from a seed colour.
pub fn generate_theme_json(seed: Color32) -> String {
    let argb = color32_to_argb(seed);
    let seed_hex = argb_to_hex(argb);

    let get_scheme = |is_dark: bool, contrast: f64| -> serde_json::Value {
        let dynamic_scheme =
            DynamicScheme::by_variant(argb, &Variant::TonalSpot, is_dark, Some(contrast));
        let scheme = Scheme::from(dynamic_scheme);
        serde_json::Value::Object(scheme_to_map(scheme))
    };

    let mut schemes = serde_json::Map::new();
    schemes.insert("light".to_string(), get_scheme(false, 0.0));
    schemes.insert("light-medium-contrast".to_string(), get_scheme(false, 0.5));
    schemes.insert("light-high-contrast".to_string(), get_scheme(false, 1.0));
    schemes.insert("dark".to_string(), get_scheme(true, 0.0));
    schemes.insert("dark-medium-contrast".to_string(), get_scheme(true, 0.5));
    schemes.insert("dark-high-contrast".to_string(), get_scheme(true, 1.0));

    let mut core_colors = serde_json::Map::new();
    core_colors.insert(
        "primary".to_string(),
        serde_json::Value::String(seed_hex.clone()),
    );

    let mut theme = serde_json::Map::new();
    theme.insert(
        "description".to_string(),
        serde_json::Value::String("Hachimi generated theme".to_string()),
    );
    theme.insert("seed".to_string(), serde_json::Value::String(seed_hex));
    theme.insert(
        "coreColors".to_string(),
        serde_json::Value::Object(core_colors),
    );
    theme.insert(
        "extendedColors".to_string(),
        serde_json::Value::Array(vec![]),
    );
    theme.insert("schemes".to_string(), serde_json::Value::Object(schemes));
    theme.insert(
        "palettes".to_string(),
        serde_json::Value::Object(serde_json::Map::new()),
    );

    serde_json::to_string_pretty(&serde_json::Value::Object(theme)).unwrap_or_default()
}
