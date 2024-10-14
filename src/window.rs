//! Handles window initialization and events, using Winit.

use std::time::Instant;

use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, WindowEvent},
    event_loop::ActiveEventLoop,
    window::{Window, WindowId},
};

use crate::{system::State, EngineUpdates, Scene};

impl<T, FRender, FEvent, FGui> State<T, FRender, FEvent, FGui>
where
    FRender: FnMut(&mut T, &mut Scene, f32) -> EngineUpdates + 'static,
    FEvent: FnMut(&mut T, DeviceEvent, &mut Scene, f32) -> EngineUpdates + 'static,
    FGui: FnMut(&mut T, &egui::Context, &mut Scene) -> EngineUpdates + 'static,
{
    fn redraw(&mut self) {
        if self.sys.is_none() || self.graphics.is_none() {
            return;
        }

        let sys = &self.sys.as_ref().unwrap();
        let graphics = &mut self.graphics.as_mut().unwrap();

        let now = Instant::now();
        self.dt = now - self.last_render_time;
        self.last_render_time = now;

        let dt_secs = self.dt.as_secs() as f32 + self.dt.subsec_micros() as f32 / 1_000_000.;
        let engine_updates =
            (self.render_handler)(&mut self.user_state, &mut graphics.scene, dt_secs);

        if engine_updates.meshes {
            graphics.setup_vertices_indices(&sys.device);
            graphics.setup_entities(&sys.device);
        }

        // Entities have been updated in the scene; update the buffers
        if engine_updates.entities {
            graphics.setup_entities(&sys.device);
        }

        if engine_updates.camera {
            // Entities have been updated in the scene; update the buffer.
            graphics.update_camera(&sys.queue);
        }

        if engine_updates.lighting {
            // Entities have been updated in the scene; update the buffer.
            graphics.update_lighting(&sys.queue);
        }

        // Note that the GUI handler can also modify entities, but
        // we do that in the `init_graphics` module.

        // todo: move this into `render`?
        match sys.surface.get_current_texture() {
            Ok(output_frame) => {
                let output_view = output_frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                let resize_required = graphics.render(
                    output_frame,
                    &output_view,
                    &sys.device,
                    &sys.queue,
                    self.dt,
                    sys.surface_cfg.width,
                    sys.surface_cfg.height,
                    &mut self.ui_settings,
                    &mut self.gui_handler,
                    &mut self.user_state,
                );

                if resize_required {
                    println!("RESIZE req"); // todo temp
                    self.resize(sys.size);
                }
            }
            // todo: Does this happen when minimized?
            Err(e) => {
                eprintln!("Error getting the current texture: {:?}", e);
            }
        }

        // todo? In the example
        // graphics.egui_renderer.end_frame_and_draw(
        //     &sys.device,
        //     &sys.queue,
        //     &mut encoder,
        //     &graphics.window,
        //     // &surface_view,
        //     // screen_descriptor,
        // );
    }
}

impl<T, FRender, FEvent, FGui> ApplicationHandler for State<T, FRender, FEvent, FGui>
where
    FRender: FnMut(&mut T, &mut Scene, f32) -> EngineUpdates + 'static,
    FEvent: FnMut(&mut T, DeviceEvent, &mut Scene, f32) -> EngineUpdates + 'static,
    FGui: FnMut(&mut T, &egui::Context, &mut Scene) -> EngineUpdates + 'static,
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = event_loop
            .create_window(Window::default_attributes())
            .unwrap();

        self.init(window);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        if self.sys.is_none() || self.graphics.is_none() {
            return;
        }

        let sys = &mut self.sys.as_mut().unwrap();
        let graphics = &mut self.graphics.as_mut().unwrap();

        // Let the EGUI renderer to process the event first. This step is required for the UI
        // to process inputs.
        let _ = graphics
            .egui_state
            .on_window_event(&graphics.window, &event);

        match event {
            WindowEvent::RedrawRequested => {
                self.redraw();
                graphics.window.as_ref().request_redraw();
            }
            WindowEvent::CursorMoved { position, .. } => {
                if position.x < self.ui_settings.size {
                    sys.mouse_in_gui = true;

                    // We reset the inputs, since otherwise a held key that
                    // doesn't get the reset command will continue to execute.
                    graphics.inputs_commanded = Default::default();
                } else {
                    sys.mouse_in_gui = false;
                }
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(physical_size) => {
                self.resize(physical_size);
                // Prevents inadvertent mouse-click-activated free-look.
                graphics.inputs_commanded.free_look = false;
                // graphics.window.resize(size); // todo??
            }
            // If the window scale changes, update the renderer size, and camera aspect ratio.
            WindowEvent::ScaleFactorChanged {
                scale_factor,
                inner_size_writer,
                ..
            } => {
                // todo: Address this.
                // self.resize(scale_factor); // todo: Changed in 2024
            }
            // If the window is being moved, disable mouse inputs, eg so click+drag
            // doesn't cause a drag when moving the window using the mouse.
            WindowEvent::Moved(_) => {
                sys.mouse_in_gui = true;
                // Prevents inadvertent mouse-click-activated free-look after moving the window.
                graphics.inputs_commanded.free_look = false;
            }
            WindowEvent::Occluded(_) => {
                // Prevents inadvertent mouse-click-activated free-look after minimizing.
                graphics.inputs_commanded.free_look = false;
            }
            WindowEvent::Focused(_) => {
                // Eg clicking the tile bar icon.
                graphics.inputs_commanded.free_look = false;
            }
            WindowEvent::CursorLeft { device_id: _ } => {
                // todo: Not working
                // graphics.inputs_commanded.free_look = false;
            }
            _ => {}
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        if self.sys.is_none() || self.graphics.is_none() {
            return;
        }

        let sys = &self.sys.unwrap();
        let graphics = &mut self.graphics.unwrap();

        if !sys.mouse_in_gui {
            let dt_secs = self.dt.as_secs() as f32 + self.dt.subsec_micros() as f32 / 1_000_000.;
            let engine_updates = (self.event_handler)(
                &mut self.user_state,
                event.clone(),
                &mut graphics.scene,
                dt_secs,
            );

            if engine_updates.meshes {
                graphics.setup_vertices_indices(&sys.device);
                graphics.setup_entities(&sys.device);
            }

            // Entities have been updated in the scene; update the buffers.
            if engine_updates.entities {
                graphics.setup_entities(&sys.device);
            }

            if engine_updates.camera {
                // Entities have been updated in the scene; update the buffer.
                graphics.update_camera(&sys.queue);
            }

            if engine_updates.lighting {
                graphics.update_lighting(&sys.queue);
            }

            graphics.handle_input(event, &self.input_settings);
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {

        // todo?
        // (self.gui_handler)(args)
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        // We must drop the context here.
        // self.context = None;
    }
}
