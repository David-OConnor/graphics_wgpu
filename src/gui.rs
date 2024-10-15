//! GUI code for EGUI, to run on the WGPU painter.
//! See [this unofficial example](https://github.com/kaphula/winit-egui-wgpu-template/tree/master/src)
//! https://github.com/rust-windowing/winit/issues/3626

use std::sync::Arc;

use egui::{ClippedPrimitive, Context, FullOutput};
use egui_wgpu::{Renderer, ScreenDescriptor};
use egui_winit;
use wgpu::{self, CommandEncoder, Device, Queue, TextureFormat};
use winit::window::Window;

use crate::{
    graphics::GraphicsState,
    system::DEPTH_FORMAT,
    types::{EngineUpdates, Scene},
    UiSettings,
};

/// State related to the GUI.
pub(crate) struct GuiState {
    pub egui_state: egui_winit::State,
    pub egui_renderer: Renderer,
    pub ui_size_prev: f64,
    /// Used to disable inputs while the mouse is in the GUI section.
    pub mouse_in_gui: bool,
}

impl GuiState {
    pub fn new(window: Arc<Window>, device: &Device, texture_format: TextureFormat) -> Self {
        let egui_context = Context::default();
        let egui_state = egui_winit::State::new(
            egui_context,
            egui::viewport::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );

        let egui_renderer = Renderer::new(
            device,
            texture_format,
            Some(DEPTH_FORMAT),
            1,     // todo
            false, // todo: Dithering?
        );

        Self {
            egui_state,
            egui_renderer,
            ui_size_prev: 0.,
            mouse_in_gui: false,
        }
    }

    /// This function contains code specific to rendering the GUI prior to the render pass.
    pub(crate) fn render_gui_pre_rpass<T>(
        &mut self,
        window: &Window,
        user_state: &mut T,
        device: &Device,
        mut gui_handler: impl FnMut(&mut T, &Context, &mut Scene) -> EngineUpdates,
        encoder: &mut CommandEncoder,
        queue: &Queue,
        width: u32,
        height: u32,
        engine_updates: &mut EngineUpdates,
    ) -> (FullOutput, Vec<ClippedPrimitive>, ScreenDescriptor) {
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [width, height],
            pixels_per_point: window.scale_factor() as f32,
        };

        self.egui_state
            .egui_ctx()
            .set_pixels_per_point(screen_descriptor.pixels_per_point);

        let raw_input = self.egui_state.take_egui_input(window);
        let full_output = self.egui_state.egui_ctx().run(raw_input, |ui| {
            // todo: Put back
            // *engine_updates = gui_handler(
            //     user_state,
            //     g_state.eself.egui_ctx(),
            //     &mut g_state.scene,
            // );
        });

        self.egui_state
            .handle_platform_output(window, full_output.platform_output.clone()); // todo: Is this clone OK?

        let tris = self.egui_state.egui_ctx().tessellate(
            full_output.shapes.clone(), // todo: Is the clone OK?
            self.egui_state.egui_ctx().pixels_per_point(),
        );

        for (id, image_delta) in &full_output.textures_delta.set {
            self.egui_renderer
                .update_texture(device, queue, *id, image_delta);
        }

        self.egui_renderer
            .update_buffers(device, queue, encoder, &tris, &screen_descriptor);

        (full_output, tris, screen_descriptor)
    }
}

/// In each render, process engine updates from the GUI handler callback, from the application.
pub(crate) fn process_engine_updates(
    g_state: &mut GraphicsState,
    ui_settings: &mut UiSettings,
    engine_updates: &EngineUpdates,
    device: &Device,
    queue: &Queue,
) {
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

    ui_settings.size = engine_updates.ui_size as f64;
}
