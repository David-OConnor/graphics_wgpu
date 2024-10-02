//! This module initiates the window, and graphics hardware.

#[cfg(not(target_arch = "wasm32"))]
use std::{
    path::Path,
    time::{Duration, Instant},
};

use image::ImageError;
use winit::{
    event::{DeviceEvent, Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Icon, Window, WindowBuilder},
};
use winit::window::{WindowAttributes, WindowId};
use crate::{
    graphics::GraphicsState,
    texture::Texture,
    types::{EngineUpdates, InputSettings, Scene, UiLayout, UiSettings},
};
use std::sync::mpsc::{self, Receiver, Sender};

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
    /// Used to disable inputs while the mouse is in the GUI section.
    pub mouse_in_gui: bool, // todo: Is this how you want to handle this?
}

pub struct State {
    pub sys: SystemState,
    pub graphics: GraphicsState,
    // Below is part of new Winit system
    // pub windows: HashMap<WindowId, WindowState>,
}

impl State {
    pub(crate) fn new(
        window: &Window,
        scene: Scene,
        input_settings: InputSettings,
        ui_settings: UiSettings,
        // compute_shader: Option<&str>, // Shader file, as a UTF-8
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
            // Note that `Fifo` locks FPS to the speed of the monitor.
            present_mode: wgpu::PresentMode::Fifo,
            // todo: Allow config from user.
            // present_mode: wgpu::PresentMode::Immediate,
            // present_mode: wgpu::PresentMode::Mailbox,
            alpha_mode: wgpu::CompositeAlphaMode::Auto, // todo?
        };

        // todo: 0.15 WGPU once it's compatible with WGPU_EGUI BACKEND:
        //                             .ok()
        //                     })
        //                     .expect("couldn't append canvas to document body");
        //             }
        //
        //         let size = window.inner_size();
        //
        //         // let backends = wgpu::util::backend_bits_from_env().unwrap_or_else(wgpu::Backends::all);
        //         let backends = wgpu::Backends::VULKAN;
        //         let dx12_shader_compiler = wgpu::util::dx12_shader_compiler_from_env().unwrap_or_default();
        //
        //         // The instance is a handle to our GPU. Its main purpose is to create Adapters and Surfaces.
        //         let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        //             backends,
        //             dx12_shader_compiler,
        //         });
        //
        //         let surface = instance.create_surface(window).unwrap();
        //
        //         let (adapter, device, queue) = pollster::block_on(setup_async(&instance, &surface));
        //
        //         // The surface is the part of the window that we draw to. We need it to draw directly to the
        //         // screen. Our window needs to implement raw-window-handle (opens new window)'s
        //         // HasRawWindowHandle trait to create a surface.
        //
        //         let surface_cfg = wgpu::SurfaceConfiguration {
        //             usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        //             format: wgpu::TextureFormat::Rgba8UnormSrgb, // todo?
        //             width: size.width,
        //             height: size.height,
        //             // https://docs.rs/wgpu/latest/wgpu/enum.PresentMode.html
        //             // Note that `Fifo` locks FPS to the speed of the monitor.
        //             present_mode: wgpu::PresentMode::Fifo,
        //             // todo: Allow config from user.
        //             // present_mode: wgpu::PresentMode::Immediate,
        //             // present_mode: wgpu::PresentMode::Mailbox,
        //             alpha_mode: wgpu::CompositeAlphaMode::Auto, // todo?
        //             view_formats: vec![wgpu::TextureFormat::Rgba32Uint], // todo?
        //         };
        //
        //         surface.configure(&device, &surface_cfg);
        //
        //         let sys = SystemState {
        //             instance,
        //             size,

        surface.configure(&device, &surface_cfg);

        let sys = SystemState {
            instance,
            size,
            surface,
            adapter,
            device,
            queue,
            surface_cfg,
            mouse_in_gui: false,
        };

        let graphics = GraphicsState::new(
            &sys.device,
            // &sys.queue,
            &sys.surface_cfg,
            scene,
            input_settings,
            ui_settings,
            window,
            // &sys.adapter,
            // compute_shader,
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

            let (eff_width, eff_height) = match self.graphics.ui_settings.layout {
                UiLayout::Left | UiLayout::Right => (
                    self.sys.surface_cfg.width as f32 - self.graphics.ui_settings.size as f32,
                    self.sys.surface_cfg.height as f32,
                ),
                _ => (
                    self.sys.surface_cfg.width as f32,
                    self.sys.surface_cfg.height as f32 - self.graphics.ui_settings.size as f32,
                ),
            };

            self.graphics.scene.camera.aspect = eff_width / eff_height;

            self.graphics.depth_texture = Texture::create_depth_texture(
                &self.sys.device,
                &self.sys.surface_cfg,
                "Depth texture",
            );

            self.graphics.scene.camera.update_proj_mat();
        }
    }
}

fn load_icon(path: &Path) -> Result<Icon, ImageError> {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::open(path)?.into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };
    Ok(Icon::from_rgba(icon_rgba, icon_width, icon_height).expect("Failed to open icon"))
}

pub fn run<T: 'static>(
    mut user_state: T,
    scene: Scene,
    input_settings: InputSettings,
    ui_settings: UiSettings,
    mut render_handler: impl FnMut(&mut T, &mut Scene, f32) -> EngineUpdates + 'static,
    mut event_handler: impl FnMut(&mut T, DeviceEvent, &mut Scene, f32) -> EngineUpdates + 'static,
    mut gui_handler: impl FnMut(&mut T, &egui::Context, &mut Scene) -> EngineUpdates + 'static,
) {
    // cfg_if::cfg_if! {
    //     if #[cfg(target_arch = "wasm32")] {
    //         std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    //         console_log::init_with_level(log::Level::Warn).expect("Couldn't initialize logger");
    //     } else {
    // }
    // }

    #[cfg(not(target_arch = "wasm32"))]
    let mut _last_frame_inst = Instant::now();
    #[cfg(not(target_arch = "wasm32"))]
    let (_frame_count, mut _accum_time) = (0, 0.0);

    let icon = match ui_settings.icon_path {
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

    let event_loop = EventLoop::new();

    let window_attributes = WindowAttributes::default()
        .with_title(WINDOW_TITLE_INIT)
        .with_inner_size(winit::dpi::LogicalSize::new(
            WINDOW_SIZE_X_INIT,
            WINDOW_SIZE_Y_INIT,
        ))
        .with_window_icon(icon)
        .build(&event_loop)
        .unwrap();

    let window = event_loop.create_window(window_attributes);

    let state = State::new(&window, scene, input_settings, ui_settings);

    event_loop.run_app(state).unwrap();

    // event_loop.run(move |event, _, control_flow| {
    //     let _ = (&state.sys.instance, &state.sys.adapter); // force ownership by the closure
    //
    //
    //     // For the GUI
    //     // Pass the winit events to the platform integration.
    //
    //
    //     match event {
    //         // WindowEvent::MainEventsCleared => window.request_redraw(),
    //         // Event::WindowEvent {
    //         //     ref event,
    //         //     window_id,
    //         //     // } if window_id == window.id() && !state.input(event) => {
    //         // } if window_id == window.id() => {
    //         //     match event {
    //         //
    //         //     }
    //         // }
    //
    //
    //         _ => {}
    //     }
    // });
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
            compatible_surface: Some(surface),
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
