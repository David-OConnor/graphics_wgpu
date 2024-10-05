//! GUI code for EGUI, to run on the WGPU painter.
//! See [this unofficial example](https://github.com/kaphula/winit-egui-wgpu-template/tree/master/src)
//! https://github.com/rust-windowing/winit/issues/3626

use egui::Context;
// use egui_wgpu_backend::{ScreenDescriptor};
use egui_wgpu::{ScreenDescriptor};
use egui_winit;
use wgpu::{self, CommandEncoder, Device, Queue, StoreOp, Surface, TextureView};
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
    // encoder: &mut CommandEncoder,
    user_state: &mut T,
    mut gui_handler: impl FnMut(&mut T, &Context, &mut Scene) -> EngineUpdates,
    output_view: &TextureView,
    // window: &Window,
    width: u32,
    height: u32,
    surface: &Surface,
// ) -> egui::TexturesDelta {
) {
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
    // let full_output = g_state.egui_state.egui_ctx().end_pass();
    // let raw_input = g_state.egui_state.take_egui_input(&g_state.window);
    // let full_output = g_state.egui_state.egui_ctx().run(raw_input, |ui| {
    //     // run_ui(g_state.egui_state.egui_ctx());
    //
    //     // todo: this?
    //     // gui_handler(state_user, g_state.egui_state.egui_ctx(), scene);
    // });
    //
    // // let paint_jobs = g_state.egui_state
    // //     .egui_ctx()
    // //     .tessellate(full_output.shapes, full_output.pixels_per_point);
    //
    // let screen_descriptor = ScreenDescriptor {
    //     size_in_pixels: [width, height],
    //     pixels_per_point: g_state.window.scale_factor() as f32,
    // };
    //
    // // let tdelta: egui::TexturesDelta = full_output.textures_delta;
    //
    // let tris = g_state.egui_state
    //     .egui_ctx()
    //     .tessellate(full_output.shapes, g_state.egui_state.egui_ctx().pixels_per_point());
    //
    // for (id, image_delta) in &full_output.textures_delta.set {
    //     g_state.egui_renderer
    //         .update_texture(device, queue, *id, image_delta);
    // }
    // g_state.egui_renderer
    //     .update_buffers(device, queue, encoder, &tris, &screen_descriptor);




    //
    //
    // let surface_texture = surface
    //     .get_current_texture()
    //     .expect("Failed to acquire next swap chain texture");
    //
    // let surface_view = surface_texture
    //     .texture
    //     .create_view(&wgpu::TextureViewDescriptor::default());
    //
    // let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
    //     color_attachments: &[Some(wgpu::RenderPassColorAttachment {
    //         view: &surface_view,
    //         resolve_target: None,
    //         ops: wgpu::Operations {
    //             load: wgpu::LoadOp::Load,
    //             store: StoreOp::Store,
    //         },
    //     })],
    //     depth_stencil_attachment: None,
    //     timestamp_writes: None,
    //     label: Some("egui main render pass"),
    //     occlusion_query_set: None,
    // });
    //
    // g_state.egui_renderer.render(&mut rpass, &tris, &screen_descriptor);
    // drop(rpass);
    //
    // for x in &full_output.textures_delta.free {
    //     g_state.egui_renderer.free_texture(x)
    // }

    //
    // g_state.egui_renderer
    //     .update_texture(device, queue, *id, image_delta)
    //     .expect("add texture ok");
    //
    // g_state.egui_renderer
    //     .update_buffers(device, queue, &paint_jobs, &screen_descriptor);

    // todo? Instead of this in graphics.rs?
    //   self.rpass_egui
    //             .remove_textures(texture_delta)
    //             .expect("remove texture ok");
    // for x in &full_output.textures_delta.free {
    //     g_state.egui_renderer.free_texture(x)
    // }

    // This `execute` step must come after the render pass. Running this function after it
    // will accomplish this.
    // g_state
    //     .rpass_egui
    //     .execute(
    //         encoder,
    //         output_view,
    //         &paint_jobs, // accepts: `egui::epaint::ClippedPrimitive` (egui_wgpu_backend) We provided it... the same.
    //         &screen_descriptor,
    //         None,
    //     )
    //     .unwrap();

    // Return `tdelta`, since we need it in the `remove_textures` step, which comes later.
    // tdelta
}
