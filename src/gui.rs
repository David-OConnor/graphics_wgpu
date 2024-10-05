//! GUI code for EGUI, to run on the WGPU painter.
//! See [this unofficial example](https://github.com/kaphula/winit-egui-wgpu-template/tree/master/src)

use egui::Context;
use egui_wgpu_backend::{ScreenDescriptor};
use egui_winit;
use wgpu::{self, CommandEncoder, Device, Queue,TextureView};
use winit::{window::Window};

use crate::{
    graphics::GraphicsState,
    types::{EngineUpdates, Scene},
};


/// Render pass code specific to the GUI.
pub(crate) fn render<T>(
    g_state: &mut GraphicsState,
    device: &Device,
    queue: &Queue,
    encoder: &mut CommandEncoder,
    user_state: &mut T,
    mut gui_handler: impl FnMut(&mut T, &Context, &mut Scene) -> EngineUpdates,
    output_view: &TextureView,
    window: &Window,
    width: u32,
    height: u32,
) -> egui::TexturesDelta {
    // Begin to draw the UI frame.
    // todo: Rem 2024
    // g_state.egui_platform.begin_frame();

    let engine_updates = gui_handler(
        user_state,
        &g_state.egui_state.egui_ctx(),
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
    let full_output = g_state.egui_state.egui_ctx().end_pass();

    let paint_jobs = g_state.egui_state
        .egui_ctx()
        .tessellate(full_output.shapes, full_output.pixels_per_point);

    // let screen_descriptor = ScreenDescriptor {
    //     size_in_pixels: [width, height],
    //     pixels_per_point: window.scale_factor() as f32,
    // };
    let screen_descriptor = ScreenDescriptor {
        physical_width: width,
        physical_height: height,
        scale_factor: window.scale_factor() as f32,
        // size_in_pixels: [width, height],
        // pixels_per_point: window.scale_factor() as f32,
    };

    let tdelta: egui::TexturesDelta = full_output.textures_delta;
    g_state
        .rpass_egui
        .add_textures(device, queue, &tdelta)
        .expect("add texture ok");
    g_state
        .rpass_egui
        .update_buffers(device, queue, &paint_jobs, &screen_descriptor);

    // todo? Instead of this in graphics.rs?
    //   self.rpass_egui
    //             .remove_textures(texture_delta)
    //             .expect("remove texture ok");
    for x in &full_output.textures_delta.free {
        g_state.egui_renderer.free_texture(x)
    }

    // This `execute` step must come after the render pass. Running this function after it
    // will accomplish this.
    g_state
        .rpass_egui
        .execute(
            encoder,
            output_view,
            &paint_jobs,
            &screen_descriptor,
            None,
        )
        .unwrap();

    // Return `tdelta`, since we need it in the `remove_textures` step, which comes later.
    tdelta
}
