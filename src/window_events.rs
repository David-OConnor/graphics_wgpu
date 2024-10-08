//! Handles window events, using Winit's system.

use std::time::{Duration, Instant};

use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::WindowId,
};

use crate::{system::State, EngineUpdates, Scene};

impl<T, FRender, FEvent, FGui> ApplicationHandler for State<T, FRender, FEvent, FGui>
where
    FRender: FnMut(&mut T, &mut Scene, f32) -> EngineUpdates + 'static,
    FEvent: FnMut(&mut T, DeviceEvent, &mut Scene, f32) -> EngineUpdates + 'static,
    FGui: FnMut(&mut T, &egui::Context, &mut Scene) -> EngineUpdates + 'static,
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {}

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        // let window = match self.windows.get_mut(&window_id) {
        //     Some(window) => window,
        //     None => return,
        // };
        // *control_flow = ControlFlow::Poll;

        // todo?
        // self.graphics.egui_renderer.handle_input(&window, &event);

        match event {
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                self.dt = now - self.last_render_time;
                self.last_render_time = now;

                let dt_secs =
                    self.dt.as_secs() as f32 + self.dt.subsec_micros() as f32 / 1_000_000.;
                let engine_updates =
                    (self.render_handler)(&mut self.user_state, &mut self.graphics.scene, dt_secs);

                if engine_updates.meshes {
                    self.graphics.setup_vertices_indices(&self.sys.device);
                    self.graphics.setup_entities(&self.sys.device);
                }

                // Entities have been updated in the scene; update the buffers
                if engine_updates.entities {
                    self.graphics.setup_entities(&self.sys.device);
                }

                if engine_updates.camera {
                    // Entities have been updated in the scene; update the buffer.
                    self.graphics.update_camera(&self.sys.queue);
                }

                if engine_updates.lighting {
                    // Entities have been updated in the scene; update the buffer.
                    self.graphics.update_lighting(&self.sys.queue);
                }

                // Note that the GUI handler can also modify entities, but
                // we do that in the `init_graphics` module.

                // todo: move this into `render`?
                // todo 2024: Temp removed; getting an error.
                match self.sys.surface.get_current_texture() {
                    Ok(output_frame) => {
                        let output_view = output_frame
                            .texture
                            .create_view(&wgpu::TextureViewDescriptor::default());

                        let resize_required = self.graphics.render(
                            output_frame,
                            &output_view,
                            &self.sys.device,
                            &self.sys.queue,
                            self.dt,
                            self.sys.surface_cfg.width,
                            self.sys.surface_cfg.height,
                            // &self.sys.surface,
                            // &self.graphics.window,
                            &mut self.gui_handler,
                            &mut self.user_state,
                            &self.sys.surface,
                        );

                        if resize_required {
                            self.resize(self.sys.size);
                        }
                    }
                    // todo: Does this happen when minimized?
                    Err(_e) => {}
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if position.x < self.graphics.ui_settings.size {
                    self.sys.mouse_in_gui = true;

                    // We reset the inputs, since otherwise a held key that
                    // doesn't get the reset command will continue to execute.
                    self.graphics.inputs_commanded = Default::default();
                } else {
                    self.sys.mouse_in_gui = false;
                }
            }
            // WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
            // WindowEvent::CloseRequested =>  ControlFlow::Exit, // todo?
            WindowEvent::Resized(physical_size) => {
                self.resize(physical_size);
                // Prevents inadvertent mouse-click-activated free-look.
                self.graphics.inputs_commanded.free_look = false;
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
                self.sys.mouse_in_gui = true;
                // Prevents inadvertent mouse-click-activated free-look after moving the window.
                self.graphics.inputs_commanded.free_look = false;
            }
            WindowEvent::Occluded(_) => {
                // Prevents inadvertent mouse-click-activated free-look after minimizing.
                self.graphics.inputs_commanded.free_look = false;
            }
            WindowEvent::Focused(_) => {
                // Eg clicking the tile bar icon.
                self.graphics.inputs_commanded.free_look = false;
            }
            WindowEvent::CursorLeft { device_id: _ } => {
                // todo: Not working
                // self.graphics.inputs_commanded.free_look = false;
            }
            _ => {}
        }
    }

    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        device_id: DeviceId,
        // device_id:  wgpu::core::id::Id<wgpu::core::id::markers::Device>,
        event: DeviceEvent,
    ) {
        // println!("EV: {:?}", event);
        if !self.sys.mouse_in_gui {
            let dt_secs = self.dt.as_secs() as f32 + self.dt.subsec_micros() as f32 / 1_000_000.;
            let engine_updates = (self.event_handler)(
                &mut self.user_state,
                event.clone(),
                &mut self.graphics.scene,
                dt_secs,
            );

            if engine_updates.meshes {
                self.graphics.setup_vertices_indices(&self.sys.device);
                self.graphics.setup_entities(&self.sys.device);
            }

            // Entities have been updated in the scene; update the buffers.
            if engine_updates.entities {
                self.graphics.setup_entities(&self.sys.device);
            }

            if engine_updates.camera {
                // Entities have been updated in the scene; update the buffer.
                self.graphics.update_camera(&self.sys.queue);
            }

            if engine_updates.lighting {
                self.graphics.update_lighting(&self.sys.queue);
            }

            self.graphics.handle_input(event);
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // if self.windows.is_empty() {
        //     event_loop.exit();
        // }

        // todo
        // (self.gui_handler)(args)
        // (self.event_handler)(args)
        // (self.render_handler_handler)(args)
    }

    #[cfg(not(android_platform))]
    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        // We must drop the context here.
        // self.context = None;
    }
}
