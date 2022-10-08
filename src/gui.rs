//! GUI code for EGUI. This code doesn't include any WGPU-specific
//! code; it's just the UI.
//! See [this official example](https://github.com/emilk/egui/tree/master/crates/egui_demo_lib)

use egui::FontDefinitions;
use egui_winit_platform::{Platform, PlatformDescriptor};
use wgpu::SurfaceConfiguration;
use winit::window::Window;

pub(crate) fn setup_platform(surface_cfg: &SurfaceConfiguration, window: &Window) -> Platform {
    Platform::new(PlatformDescriptor {
        physical_width: surface_cfg.width,
        // todo: Respect the UI placement in `ui_settings`.
        physical_height: surface_cfg.height,
        scale_factor: window.scale_factor(),
        font_definitions: FontDefinitions::default(),
        style: Default::default(),
    })
}
