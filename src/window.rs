//! Handles window initialization and events, using Winit.

use std::{path::Path, time::Instant};

use image::ImageError;
use wgpu::TextureViewDescriptor;
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, WindowEvent},
    event_loop::ActiveEventLoop,
    window::{Icon, Window, WindowAttributes, WindowId},
};

use crate::{system::State, EngineUpdates, Scene};

const WINDOW_TITLE_INIT: &str = "Graphics";
const WINDOW_SIZE_X_INIT: f32 = 900.0;
const WINDOW_SIZE_Y_INIT: f32 = 600.0;

fn load_icon(path: &Path) -> Result<Icon, ImageError> {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::open(path)?.into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };
    Ok(Icon::from_rgba(icon_rgba, icon_width, icon_height).expect("Failed to open icon"))
}

impl<T, FRender, FEvent, FGui> State<T, FRender, FEvent, FGui>
where
    FRender: FnMut(&mut T, &mut Scene, f32) -> EngineUpdates + 'static,
    FEvent: FnMut(&mut T, DeviceEvent, &mut Scene, f32) -> EngineUpdates + 'static,
    FGui: FnMut(&mut T, &egui::Context, &mut Scene) -> EngineUpdates + 'static,
{
    fn redraw(&mut self) {
        if self.render.is_none() || self.graphics.is_none() {
            return;
        }

        let sys = &self.render.as_ref().unwrap();
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
                    .create_view(&TextureViewDescriptor::default());

                let resize_required = graphics.render(
                    &mut self.gui.as_mut().unwrap(),
                    output_frame,
                    &output_view,
                    &sys.device,
                    &sys.queue,
                    self.dt,
                    sys.surface_cfg.width,
                    sys.surface_cfg.height,
                    &mut self.ui_settings,
                    &self.input_settings,
                    &mut self.gui_handler,
                    &mut self.user_state,
                );

                if resize_required {
                    println!("Resize requested from GUI");
                    self.resize(sys.size);
                }
            }
            // This occurs when minimized.
            Err(_e) => (),
        }
    }
}

impl<T, FRender, FEvent, FGui> ApplicationHandler for State<T, FRender, FEvent, FGui>
where
    FRender: FnMut(&mut T, &mut Scene, f32) -> EngineUpdates + 'static,
    FEvent: FnMut(&mut T, DeviceEvent, &mut Scene, f32) -> EngineUpdates + 'static,
    FGui: FnMut(&mut T, &egui::Context, &mut Scene) -> EngineUpdates + 'static,
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        println!("Engine resumed; rebuilding window, render, and graphics state.");
        // todo: Only re-init if not already inited?

        let icon = match self.ui_settings.icon_path {
            Some(ref p) => {
                match load_icon(Path::new(&p)) {
                    Ok(p_) => Some(p_),
                    // eg can't find the path
                    Err(_) => None,
                }
            }
            // No path specified
            None => None,
        };

        let attributes = WindowAttributes::default()
            .with_title(WINDOW_TITLE_INIT)
            .with_inner_size(winit::dpi::LogicalSize::new(
                WINDOW_SIZE_X_INIT,
                WINDOW_SIZE_Y_INIT,
            ))
            .with_window_icon(icon);

        let window = event_loop.create_window(attributes).unwrap();

        self.init(window);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        if self.render.is_none() || self.graphics.is_none() {
            return;
        }

        let graphics = &mut self.graphics.as_mut().unwrap();
        let gui = &mut self.gui.as_mut().unwrap();

        //     if let Some(gui) = self.gui.as_mut() {
        //     if let Some(graphics) = self.graphics.as_mut() {
        //         let window = &graphics.window;
        //         let _ = gui.egui_state.on_window_event(window, &event);
        //     }
        // }

        let window = &graphics.window;
        let _ = gui.egui_state.on_window_event(window, &event);

        match event {
            WindowEvent::RedrawRequested => {
                self.redraw();
                self.graphics.as_ref().unwrap().window.request_redraw();
            }
            WindowEvent::CursorMoved { position, .. } => {
                if position.x < self.ui_settings.size {
                    gui.mouse_in_gui = true;

                    // We reset the inputs, since otherwise a held key that
                    // doesn't get the reset command will continue to execute.
                    self.graphics.as_mut().unwrap().inputs_commanded = Default::default();
                } else {
                    gui.mouse_in_gui = false;
                }
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(physical_size) => {
                self.resize(physical_size);
                // Prevents inadvertent mouse-click-activated free-look.
                self.graphics.as_mut().unwrap().inputs_commanded.free_look = false;
            }
            // If the window scale changes, update the renderer size, and camera aspect ratio.
            WindowEvent::ScaleFactorChanged {
                scale_factor,
                inner_size_writer,
                ..
            } => {
                // Note: This appears to not come up, nor is it required. (Oct 2024)
                println!("Scale factor changed");
            }
            // If the window is being moved, disable mouse inputs, eg so click+drag
            // doesn't cause a drag when moving the window using the mouse.
            WindowEvent::Moved(_) => {
                gui.mouse_in_gui = true;
                // Prevents inadvertent mouse-click-activated free-look after moving the window.
                self.graphics.as_mut().unwrap().inputs_commanded.free_look = false;
            }
            WindowEvent::Occluded(_) => {
                // Prevents inadvertent mouse-click-activated free-look after minimizing.
                self.graphics.as_mut().unwrap().inputs_commanded.free_look = false;
            }
            WindowEvent::Focused(_) => {
                // Eg clicking the tile bar icon.
                self.graphics.as_mut().unwrap().inputs_commanded.free_look = false;
            }
            WindowEvent::CursorLeft { device_id: _ } => {
                // todo: Not working?
                graphics.inputs_commanded.free_look = false;
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
        if self.render.is_none() || self.graphics.is_none() {
            return;
        }

        let sys = &self.render.as_ref().unwrap();
        let graphics = &mut self.graphics.as_mut().unwrap();
        let gui = &mut self.gui.as_mut().unwrap();

        if !gui.mouse_in_gui {
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
