use crate::core::gui::config::ConfigEditor;
#[allow(unused_imports)]
use crate::core::gui::utils::grid_control_w;
#[allow(unused_imports)]
use egui_material3::*;
use rust_i18n::t;
use crate::il2cpp::hook::umamusume::CameraData::ShadowResolution;
use crate::il2cpp::hook::umamusume::GraphicSettings::{GraphicsQuality, MsaaQuality};
use crate::il2cpp::hook::UnityEngine_CoreModule::Texture::AnisoLevel;


#[allow(unused_variables)]
pub fn render(editor: &ConfigEditor, config: &mut crate::core::hachimi::Config, ui: &mut egui::Ui) {

    ConfigEditor::list_tile_option_slider(ui, &t!("config_editor.target_fps"), &mut config.target_fps, 30..=240);

    // Unfocused FPS cap — independent of the focused Target FPS option.
    #[cfg(target_os = "windows")]
    {
        ConfigEditor::list_tile_option_slider(
            ui, &t!("config_editor.target_fps_unfocused"), &mut config.windows.target_fps_unfocused, 1..=240
        );
    }
    ConfigEditor::list_tile_slider(ui, t!("config_editor.virtual_resolution_multiplier"), &mut config.virtual_res_mult, 1.0..=4.0, 0.1, 1);
    ConfigEditor::list_tile_slider(ui, t!("config_editor.ui_scale"), &mut config.ui_scale, 0.1..=10.0, 0.05, 2);
    ConfigEditor::list_tile_slider(ui, t!("config_editor.ui_animation_scale"), &mut config.ui_animation_scale, 0.1..=10.0, 0.1, 1);
    ConfigEditor::list_tile_slider(ui, t!("config_editor.render_scale"), &mut config.render_scale, 0.1..=10.0, 0.1, 1);

    ConfigEditor::list_tile_combo(ui, t!("config_editor.msaa"), "msaa", &mut config.msaa, &[
        (MsaaQuality::Disabled, &t!("default")),
        (MsaaQuality::_2x, "2x"), (MsaaQuality::_4x, "4x"), (MsaaQuality::_8x, "8x"),
    ]);
    ConfigEditor::list_tile_combo(ui, t!("config_editor.aniso_level"), "aniso_level", &mut config.aniso_level, &[
        (AnisoLevel::Default, &t!("default")),
        (AnisoLevel::_2x, "2x"), (AnisoLevel::_4x, "4x"),
        (AnisoLevel::_8x, "8x"), (AnisoLevel::_16x, "16x"),
    ]);
    ConfigEditor::list_tile_combo(ui, t!("config_editor.shadow_resolution"), "shadow_resolution", &mut config.shadow_resolution, &[
        (ShadowResolution::Default, &t!("default")),
        (ShadowResolution::_256, "256x"), (ShadowResolution::_512, "512x"),
        (ShadowResolution::_1024, "1K"), (ShadowResolution::_2048, "2K"), (ShadowResolution::_4096, "4K"),
    ]);
    ConfigEditor::list_tile_combo(ui, t!("config_editor.graphics_quality"), "graphics_quality", &mut config.graphics_quality, &[
        (GraphicsQuality::Default, &t!("default")),
        (GraphicsQuality::Toon1280, "Toon1280"), (GraphicsQuality::Toon1280x2, "Toon1280x2"),
        (GraphicsQuality::Toon1280x4, "Toon1280x4"), (GraphicsQuality::ToonFull, "ToonFull"),
        (GraphicsQuality::Max, "Max"),
    ]);

    #[cfg(target_os = "windows")]
    {
        use crate::windows::hachimi_impl::{FullScreenMode, ResolutionScaling};
        ConfigEditor::list_tile_combo(ui, t!("config_editor.vsync"), "vsync", &mut config.windows.vsync_count, &[
            (-1, &t!("default")), (0, &t!("off")), (1, &t!("on")), (2, "1/2"), (3, "1/3"), (4, "1/4"),
        ]);
        ConfigEditor::list_tile_switch(ui, t!("config_editor.auto_full_screen"), &mut config.windows.auto_full_screen, true);
        ConfigEditor::list_tile_combo(ui, t!("config_editor.full_screen_mode"), "full_screen_mode", &mut config.windows.full_screen_mode, &[
            (FullScreenMode::ExclusiveFullScreen, &t!("config_editor.full_screen_mode_exclusive")),
            (FullScreenMode::FullScreenWindow,    &t!("config_editor.full_screen_mode_borderless")),
        ]);
        ConfigEditor::list_tile_switch(ui, t!("config_editor.block_minimize_in_full_screen"), &mut config.windows.block_minimize_in_full_screen, true);
        ConfigEditor::list_tile_combo(ui, t!("config_editor.resolution_scaling"), "resolution_scaling", &mut config.windows.resolution_scaling, &[
            (ResolutionScaling::Default,         &t!("config_editor.resolution_scaling_default")),
            (ResolutionScaling::ScaleToScreenSize, &t!("config_editor.resolution_scaling_ssize")),
            (ResolutionScaling::ScaleToWindowSize, &t!("config_editor.resolution_scaling_wsize")),
        ]);
        ConfigEditor::list_tile_switch(ui, t!("config_editor.window_always_on_top"), &mut config.windows.window_always_on_top, true);

        ConfigEditor::list_tile_switch(ui, t!("config_editor.freeform_window"), &mut config.windows.freeform_window, true);
        ConfigEditor::list_tile_switch(ui, t!("config_editor.freeform_ui_scale_auto"), &mut config.windows.freeform_ui_scale_auto, true);
        if config.windows.freeform_ui_scale_auto {
            ConfigEditor::list_tile_slider(ui, t!("config_editor.freeform_ui_scale_auto_ratio"),
                &mut config.windows.freeform_ui_scale_auto_ratio, 0.25..=3.0, 0.05, 2);
        }

        ConfigEditor::list_tile_switch(ui, t!("config_editor.ui_loading_show_orientation_guide"),
            &mut config.windows.ui_loading_show_orientation_guide, true);

        if !ConfigEditor::row_filtered(&t!("config_editor.full_screen_res")) {
            let is_landscape = ui.ctx().input(|i| i.viewport_rect().width() > i.viewport_rect().height());
            let label_text = if is_landscape {
                format!("{} ({})", t!("config_editor.full_screen_res"), t!("config_editor.landscape"))
            } else {
                format!("{} ({})", t!("config_editor.full_screen_res"), t!("config_editor.portrait"))
            };
            ui.add(egui::Label::new(label_text).wrap());

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;

                let scale = crate::core::gui::utils::get_scale(ui.ctx());
                let number_w = 48.0 * scale;

                if is_landscape {
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(number_w, 32.0), egui::Sense::hover());
                    let _ = ui.put(
                        rect,
                        MaterialNumberField::filled(&mut config.windows.full_screen_res.width)
                            .range(0..=7680)
                            .decimals(0),
                    );
                    ui.label("W px");
                    let (rect2, _) = ui.allocate_exact_size(egui::vec2(number_w, 32.0), egui::Sense::hover());
                    let _ = ui.put(
                        rect2,
                        MaterialNumberField::filled(&mut config.windows.full_screen_res.height)
                            .range(0..=4320)
                            .decimals(0),
                    );
                    ui.label("H px");
                } else {
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(number_w, 32.0), egui::Sense::hover());
                    let _ = ui.put(
                        rect,
                        MaterialNumberField::filled(&mut config.windows.full_screen_res.height)
                            .range(0..=4320)
                            .decimals(0),
                    );
                    ui.label("H px");
                    let (rect2, _) = ui.allocate_exact_size(egui::vec2(number_w, 32.0), egui::Sense::hover());
                    let _ = ui.put(
                        rect2,
                        MaterialNumberField::filled(&mut config.windows.full_screen_res.width)
                            .range(0..=7680)
                            .decimals(0),
                    );
                    ui.label("W px");
                }
            });
            ConfigEditor::space(ui, 4.0);
        }
    }

    #[cfg(target_os = "android")]
    {
        ConfigEditor::list_tile_combo(ui, t!("config_editor.force_orientation_mode"), "force_orientation_mode", &mut config.android.force_orientation_mode, &[
            (crate::il2cpp::types::ScreenOrientation_Unknown, t!("disabled").as_ref()),
            (crate::il2cpp::types::ScreenOrientation_LandscapeLeft, t!("config_editor.landscape").as_ref()),
            (crate::il2cpp::types::ScreenOrientation_Portrait, t!("config_editor.portrait").as_ref()),
        ]);
    }
}
