//! GUI code for EGUI, to run on the WGPU painter.
//! See [this unofficial example](https://github.com/kaphula/winit-egui-wgpu-template/tree/master/src)
//! https://github.com/rust-windowing/winit/issues/3626

use egui::{ClippedPrimitive, Context, FullOutput};
use egui_wgpu::ScreenDescriptor;
use egui_winit;
use wgpu::{self, CommandEncoder, Device, Queue, Surface, TextureView};

use crate::{
    graphics::GraphicsState,
    types::{EngineUpdates, Scene},
};

/// This function contains code specific to rendering the GUI prior to the render pass.
pub(crate) fn render_gui_pre_rpass<T>(
    g_state: &mut GraphicsState,
    user_state: &mut T,
    device: &Device,
    // mut gui_handler: impl FnMut(&mut T, &Context, &mut Scene) -> EngineUpdates,
    encoder: &mut CommandEncoder,
    queue: &Queue,
    width: u32,
    height: u32,
) -> (FullOutput, Vec<ClippedPrimitive>, ScreenDescriptor) {
    let raw_input = g_state.egui_state.take_egui_input(&g_state.window);
    let full_output = g_state.egui_state.egui_ctx().run(raw_input, |ui| {
        // todo: GUI handler here or below?
        // gui_handler(
        //     user_state,
        //     g_state.egui_state.egui_ctx(),
        //     &mut g_state.scene,
        // );
    });

    let tris = g_state.egui_state.egui_ctx().tessellate(
        full_output.shapes.clone(), // todo: Is the clone OK?
        g_state.egui_state.egui_ctx().pixels_per_point(),
    );

    let screen_descriptor = ScreenDescriptor {
        size_in_pixels: [width, height],
        pixels_per_point: g_state.window.scale_factor() as f32,
    };

    for (id, image_delta) in &full_output.textures_delta.set {
        g_state
            .egui_renderer
            .update_texture(device, queue, *id, image_delta);
    }


    g_state
        .egui_renderer
        .update_buffers(device, queue, encoder, &tris, &screen_descriptor);

    (full_output, tris, screen_descriptor)
}

/// In each render, process engine updates from the GUI handler callback, from the application.
pub(crate) fn process_engine_updates<T>(
    g_state: &mut GraphicsState,
    device: &Device,
    queue: &Queue,
    user_state: &mut T,
    mut gui_handler: impl FnMut(&mut T, &Context, &mut Scene) -> EngineUpdates,
) {
    // todo: Here, or above?
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
}
