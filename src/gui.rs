//! GUI code for EGUI. This code doesn't include anything WGPU-specific; it's just the UI.
//! See [this official example](https://github.com/emilk/egui/tree/master/crates/egui_demo_lib)

use egui::FontDefinitions;
use egui_wgpu_backend::{RenderPass, ScreenDescriptor};
use egui_winit_platform::{Platform, PlatformDescriptor};
use wgpu::{self, SurfaceConfiguration};
use winit::window::Window;

use crate::types::{EngineUpdates, Scene};

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

/// Render pass code specific to the GUI.
pub(crate) fn render<T>(
    g_state: &mut crate::graphics::GraphicsState,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    encoder: &mut wgpu::CommandEncoder,
    user_state: &mut T,
    mut gui_handler: impl FnMut(&mut T, &egui::Context, &mut Scene) -> EngineUpdates,
    output_view: &wgpu::TextureView,
    window: &Window,
    width: u32,
    height: u32,
) -> egui::TexturesDelta {
    // Begin to draw the UI frame.
    g_state.egui_platform.begin_frame();

    let engine_updates = gui_handler(
        user_state,
        &mut g_state.egui_platform.context(),
        &mut g_state.scene,
    );

    if engine_updates.meshes {
        g_state.setup_vertices_indices(device);
        g_state.setup_entities(device);
    }

    if engine_updates.entities {
        g_state.setup_entities(device);
    }

    if engine_updates.camera {
        // Entities have been updated in the scene; update the buffer.
        g_state.update_camera(queue);
    }

    if engine_updates.lighting {
        // Entities have been updated in the scene; update the buffer.
        g_state.update_lighting(queue);
    }

    g_state.ui_settings.size = engine_updates.ui_size as f64;

    // End the UI frame. We could now handle the output and draw the UI with the backend.
    let full_output = g_state.egui_platform.end_frame(Some(window));
    let paint_jobs = g_state
        .egui_platform
        .context()
        .tessellate(full_output.shapes);

    // Screep descriptor for the GUI.
    let screen_descriptor = ScreenDescriptor {
        physical_width: width,
        // todo: Respect ui settings placement field.
        physical_height: height,
        scale_factor: window.scale_factor() as f32,
    };

    let tdelta: egui::TexturesDelta = full_output.textures_delta;
    g_state
        .rpass_egui
        .add_textures(device, queue, &tdelta)
        .expect("add texture ok");
    g_state
        .rpass_egui
        .update_buffers(device, queue, &paint_jobs, &screen_descriptor);

    // This `execute` step must come after the render pass. Running this function after it
    // will accomplish this.
    g_state.rpass_egui
        .execute(
            encoder,
            output_view,
            &paint_jobs,
            &screen_descriptor,
            // None here
            None,
        )
        .unwrap();

    // Return `tdelta`, since we need it in the `remove_textures` step, which comes later.
    tdelta
}
