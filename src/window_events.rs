//! Handles window events, using Winit's system.

use std::time::{Duration, Instant};
use wgpu::core::id::DeviceId;
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, WindowEvent};
use crate::system::State;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::WindowId;


impl ApplicationHandler for State {

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {

    }

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

        self.graphics.egui_platform.handle_event(&event);

        let mut last_render_time = Instant::now();

        match event {
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                dt = now - last_render_time;
                last_render_time = now;

                let dt_secs = dt.as_secs() as f32 + dt.subsec_micros() as f32 / 1_000_000.;
                let engine_updates =
                    render_handler(&mut user_state, &mut self.graphics.scene, dt_secs);

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

                // if engine_updates.compute {
                //     // Entities have been updated in the scene; update the buffer.
                //     self.graphics.compute(&self.sys.device, &self.sys.queue);
                // }

                // Note that the GUI handler can also modify entities, but
                // we do that in the `init_graphics` module.

                // todo: move this into `render`?
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
                            dt,
                            self.sys.surface_cfg.width,
                            self.sys.surface_cfg.height,
                            // &self.sys.surface,
                            &window,
                            &mut gui_handler,
                            &mut user_state,
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
                self.resize(*physical_size);
                // Prevents inadvertent mouse-click-activated free-look.
                self.graphics.inputs_commanded.free_look = false;
            }
            // If the window scale changes, update the renderer size, and camera aspect ratio.
            WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                self.resize(**new_inner_size);
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
        _event_loop: &ActiveEventLoop,
        device_id: Option<DeviceId>,
        event: DeviceEvent,
    ) {
        let mut dt = Duration::new(0, 0);

        // println!("EV: {:?}", event);
        if !self.sys.mouse_in_gui {
            let dt_secs = dt.as_secs() as f32 + dt.subsec_micros() as f32 / 1_000_000.;
            let engine_updates = event_handler(
                &mut user_state,
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
    }

    #[cfg(not(android_platform))]
    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        // We must drop the context here.
        // self.context = None;
    }
}