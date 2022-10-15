//! This module initiates the window, and graphics hardware.

#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant};

use winit::{
    event::{DeviceEvent, Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use env_logger;

use crate::{
    init_graphics::GraphicsState,
    texture::Texture,
    types::{InputSettings, Scene, UiSettings},
};

const WINDOW_TITLE_INIT: &str = "Graphics";
const WINDOW_SIZE_X_INIT: f32 = 900.0;
const WINDOW_SIZE_Y_INIT: f32 = 600.0;

pub(crate) struct SystemState {
    pub instance: wgpu::Instance,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub surface: wgpu::Surface,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface_cfg: wgpu::SurfaceConfiguration,
}

struct State {
    sys: SystemState,
    graphics: GraphicsState,
}

impl State {
    pub(crate) fn new(
        window: &Window,
        scene: Scene,
        input_settings: InputSettings,
        ui_settings: UiSettings,
    ) -> Self {
        #[cfg(target_arch = "wasm32")]
        {
            use winit::platform::web::WindowExtWebSys;
            let query_string = web_sys::window().unwrap().location().search().unwrap();
            let level: log::Level = parse_url_query_string(&query_string, "RUST_LOG")
                .map(|x| x.parse().ok())
                .flatten()
                .unwrap_or(log::Level::Error);
            console_log::init_with_level(level).expect("could not initialize logger");
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            // On wasm, append the canvas to the document body
            web_sys::window()
                .and_then(|win| win.document())
                .and_then(|doc| doc.body())
                .and_then(|body| {
                    body.append_child(&web_sys::Element::from(window.canvas()))
                        .ok()
                })
                .expect("couldn't append canvas to document body");
        }

        let size = window.inner_size();

        // The instance is a handle to our GPU. Its main purpose is to create Adapters and Surfaces.
        let instance = wgpu::Instance::new(wgpu::Backends::VULKAN);

        let surface = unsafe { instance.create_surface(window) };

        let (adapter, device, queue) = pollster::block_on(setup_async(&instance, &surface));

        // The surface is the part of the window that we draw to. We need it to draw directly to the
        // screen. Our window needs to implement raw-window-handle (opens new window)'s
        // HasRawWindowHandle trait to create a surface.
        let surface_cfg = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_supported_formats(&adapter)[0],
            width: size.width,
            height: size.height,
            // https://docs.rs/wgpu/latest/wgpu/enum.PresentMode.html
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto, // todo?
        };

        surface.configure(&device, &surface_cfg);

        let sys = SystemState {
            instance,
            size,
            surface,
            adapter,
            device,
            queue,
            surface_cfg,
        };

        let graphics = GraphicsState::new(
            &sys.device,
            // &sys.queue,
            &sys.surface_cfg,
            scene,
            input_settings,
            ui_settings,
            &window,
            // &sys.adapter,
        );

        Self { sys, graphics }
    }

    pub(crate) fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.sys.size = new_size;
            self.sys.surface_cfg.width = new_size.width;
            self.sys.surface_cfg.height = new_size.height;
            self.sys
                .surface
                .configure(&self.sys.device, &self.sys.surface_cfg);

            self.graphics.scene.camera.aspect =
                self.sys.surface_cfg.width as f32 / self.sys.surface_cfg.height as f32;

            self.graphics.depth_texture = Texture::create_depth_texture(
                &self.sys.device,
                &self.sys.surface_cfg,
                "Depth texture",
            );

            self.graphics.scene.camera.update_proj_mat();
        }
    }
}

pub fn run<'a, T: 'static>(
    mut user_state: T,
    scene: Scene,
    input_settings: InputSettings,
    ui_settings: UiSettings,
    // mut render_handler: Box<dyn FnMut(&mut Scene) -> bool>,
    // mut event_handler: Box<dyn FnMut(DeviceEvent, &mut Scene, f32) -> bool>,
    // mut gui_handler: Box<dyn FnMut(&egui::Context)>,
    mut render_handler: impl FnMut(&mut T, &mut Scene) -> bool + 'static,
    mut event_handler: impl FnMut(&mut T, DeviceEvent, &mut Scene, f32) -> bool + 'static,
    mut gui_handler: impl FnMut(&mut T, &egui::Context) + 'static,
) {
    // cfg_if::cfg_if! {
    //     if #[cfg(target_arch = "wasm32")] {
    //         std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    //         console_log::init_with_level(log::Level::Warn).expect("Couldn't initialize logger");
    //     } else {
    // `env_logger` is required to print shader errors to the console.
    // todo??
    env_logger::init();
        // }
    // }

    #[cfg(not(target_arch = "wasm32"))]
    let mut _last_frame_inst = Instant::now();
    #[cfg(not(target_arch = "wasm32"))]
    let (_frame_count, mut _accum_time) = (0, 0.0);

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title(WINDOW_TITLE_INIT)
        .with_inner_size(winit::dpi::LogicalSize::new(
            WINDOW_SIZE_X_INIT,
            WINDOW_SIZE_Y_INIT,
        ))
        .build(&event_loop)
        .unwrap();

    let mut state = State::new(&window, scene, input_settings, ui_settings);

    let mut last_render_time = Instant::now();
    let mut dt = Duration::new(0, 0);

    event_loop.run(move |event, _, control_flow| {
        let _ = (&state.sys.instance, &state.sys.adapter); // force ownership by the closure
        *control_flow = ControlFlow::Poll;

        // For the GUI
        // Pass the winit events to the platform integration.
        state.graphics.egui_platform.handle_event(&event);

        match event {
            Event::MainEventsCleared => window.request_redraw(),
            Event::DeviceEvent { event, .. } => {
                let dt_secs = dt.as_secs() as f32 + dt.subsec_micros() as f32 / 1_000_000.;
                let entities_changed =
                    event_handler(&mut user_state, event.clone(), &mut state.graphics.scene, dt_secs);

                // Entities have been updated in the scene; update the buffers
                if entities_changed {
                    state.graphics.setup_entities(&state.sys.device);
                }

                state.graphics.handle_input(event);
            }
            Event::WindowEvent {
                ref event,
                window_id,
                // } if window_id == window.id() && !state.input(event) => {
            } if window_id == window.id() => {
                match event {
                    // todo: Put back for window-closing.
                    // #[cfg(not(target_arch="wasm32"))]
                    WindowEvent::CloseRequested
                    // | WindowEvent::KeyboardInput {
                    //     input:
                    //     KeyboardInput {
                    //         state: ElementState::Pressed,
                    //         virtual_keycode: Some(VirtualKeyCode::Escape),
                    //         ..
                    //     },
                    //     ..
                    => *control_flow = ControlFlow::Exit,
                    WindowEvent::Resized(physical_size) => {
                        state.resize(*physical_size);
                    }
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        state.resize(**new_inner_size);
                    }
                    _ => {}
                }
            }

            Event::RedrawRequested(window_id) if window_id == window.id() => {
                let now = Instant::now();
                dt = now - last_render_time;
                last_render_time = now;

                let entities_changed = render_handler(&mut user_state, &mut state.graphics.scene);

                // Entities have been updated in the scene; update the buffers
                if entities_changed {
                    state.graphics.setup_entities(&state.sys.device);
                }

                // todo: move this into `render`?
                let output_frame = state.sys.surface.get_current_texture().unwrap();
                let output_view = output_frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                state.graphics.render(
                    output_frame,
                    &output_view,
                    &state.sys.device,
                    &state.sys.queue,
                    dt,
                    state.sys.surface_cfg.width,
                    state.sys.surface_cfg.height,
                    // &state.sys.surface,
                    &window,
                    &mut gui_handler,
                    &mut user_state,
                );
            }
            _ => {}
        }
    });
}

#[cfg(target_arch = "wasm32")]
pub fn run<E: Example>(title: &str) {
    use wasm_bindgen::{prelude::*, JsCast};

    let title = title.to_owned();
    wasm_bindgen_futures::spawn_local(async move {
        let setup = setup::<E>(&title).await;
        let start_closure = Closure::once_into_js(move || start::<E>(setup));

        // make sure to handle JS exceptions thrown inside start.
        // Otherwise wasm_bindgen_futures Queue would break and never handle any tasks again.
        // This is required, because winit uses JS exception for control flow to escape from `run`.
        if let Err(error) = call_catch(&start_closure) {
            let is_control_flow_exception = error.dyn_ref::<js_sys::Error>().map_or(false, |e| {
                e.message().includes("Using exceptions for control flow", 0)
            });

            if !is_control_flow_exception {
                web_sys::console::error_1(&error);
            }
        }

        #[wasm_bindgen]
        extern "C" {
            #[wasm_bindgen(catch, js_namespace = Function, js_name = "prototype.call.call")]
            fn call_catch(this: &JsValue) -> Result<(), JsValue>;
        }
    });
}

#[cfg(target_arch = "wasm32")]
/// Parse the query string as returned by `web_sys::window()?.location().search()?` and get a
/// specific key out of it.
pub(crate) fn parse_url_query_string<'a>(query: &'a str, search_key: &str) -> Option<&'a str> {
    let query_string = query.strip_prefix('?')?;

    for pair in query_string.split('&') {
        let mut pair = pair.split('=');
        let key = pair.next()?;
        let value = pair.next()?;

        if key == search_key {
            return Some(value);
        }
    }

    None
}

/// Quarantine for the Async part of the API
async fn setup_async(
    instance: &wgpu::Instance,
    surface: &wgpu::Surface,
) -> (wgpu::Adapter, wgpu::Device, wgpu::Queue) {
    // The adapter is a handle to our actual graphics card. You can use this to get
    // information about the graphics card such as its name and what backend the
    // adapter uses. We use this to create our Device and Queue.
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            // `Default` prefers low power when on battery, high performance when on mains.
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .unwrap();

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                // https://docs.rs/wgpu/latest/wgpu/struct.Features.html
                features: wgpu::Features::empty(),
                // https://docs.rs/wgpu/latest/wgpu/struct.Limits.html
                limits: wgpu::Limits::default(),
            },
            std::env::var("WGPU_TRACE")
                .ok()
                .as_ref()
                .map(std::path::Path::new),
        )
        .await
        .expect("Unable to find a suitable GPU adapter. :(");

    (adapter, device, queue)
}
