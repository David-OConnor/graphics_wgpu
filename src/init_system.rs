//! This module initiates the window, and graphics hardware.

#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant};

use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use crate::init_graphics::State;

const WINDOW_TITLE: &str = "Graphics";
const WINDOW_SIZE_X: f32 = 800.0;
const WINDOW_SIZE_Y: f32 = 800.0;

pub struct GraphicsSystem {
    instance: wgpu::Instance,
    size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_cfg: wgpu::SurfaceConfiguration,
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
        .expect("Unable to find a suitable GPU adapter!");

    (adapter, device, queue)
}

impl GraphicsSystem {
    pub fn new(window: &Window) -> GraphicsSystem {
        #[cfg(not(target_arch = "wasm32"))]
        {
            env_logger::init();
        };

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
        };

        Self {
            instance,
            size,
            surface,
            adapter,
            device,
            queue,
            surface_cfg,
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.surface_cfg.width = new_size.width;
            self.surface_cfg.height = new_size.height;
            self.surface.configure(&self.device, &self.surface_cfg);
        }
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        false
    }
}

pub fn run() {
    #[cfg(not(target_arch = "wasm32"))]
    let mut last_update_inst = Instant::now();
    #[cfg(not(target_arch = "wasm32"))]
    let mut last_frame_inst = Instant::now();
    #[cfg(not(target_arch = "wasm32"))]
    let (mut frame_count, mut accum_time) = (0, 0.0);

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title(WINDOW_TITLE)
        .with_inner_size(winit::dpi::LogicalSize::new(WINDOW_SIZE_X, WINDOW_SIZE_Y))
        .build(&event_loop).unwrap();

    let mut sys = GraphicsSystem::new(&window);
    let mut state = State::new(&sys.device, &sys.queue, &sys.surface_cfg);

    sys.surface.configure(&sys.device, &sys.surface_cfg);

    event_loop.run(move |event, _, control_flow| {
        let _ = (&sys.instance, &sys.adapter); // force ownership by the closure
        *control_flow = ControlFlow::Poll;

        match event {
            Event::RedrawEventsCleared => {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    // Clamp to some max framerate to avoid busy-looping too much
                    // (we might be in wgpu::PresentMode::Mailbox, thus discarding superfluous frames)
                    //
                    // winit has window.current_monitor().video_modes() but that is a list of all full screen video modes.
                    // So without extra dependencies it's a bit tricky to get the max refresh rate we can run the window on.
                    // Therefore we just go with 60fps - sorry 120hz+ folks!
                    let target_frametime = Duration::from_secs_f64(1.0 / 120.0);
                    let time_since_last_frame = last_update_inst.elapsed();
                    if time_since_last_frame >= target_frametime {
                        window.request_redraw();
                    } else {
                        *control_flow = ControlFlow::WaitUntil(
                            Instant::now() + target_frametime - time_since_last_frame,
                        );
                    }
                }

                #[cfg(target_arch = "wasm32")]
                window.request_redraw();
            }
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::Resized(physical_size) => {
                    sys.resize(physical_size);
                }
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    sys.resize(*new_inner_size);
                }
                _ => (),
            },
            Event::DeviceEvent { event, .. } => {
                // todo: Evaluate how you handle DT; this is quick +dirty
                let dt = last_frame_inst.elapsed().as_secs_f32();
                state.update(event, &sys.queue);
            }
            Event::RedrawRequested(_) => {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    accum_time += last_frame_inst.elapsed().as_secs_f32();
                    last_frame_inst = Instant::now();
                    frame_count += 1;
                    if frame_count == 100 {
                        // println!(
                        //     "Avg frame time {}ms",
                        //     accum_time * 1000.0 / frame_count as f32
                        // );
                        accum_time = 0.0;
                        frame_count = 0;
                    }
                }

                let frame = match sys.surface.get_current_texture() {
                    Ok(frame) => frame,
                    Err(_) => {
                        sys.surface.configure(&sys.device, &sys.surface_cfg);
                        sys.surface
                            .get_current_texture()
                            .expect("Failed to acquire next surface texture!")
                    }
                };
                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                state.render(&view, &sys.device, &sys.queue);

                frame.present();
            }
            _ => {}
        }
    });
    last_update_inst = Instant::now();
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
pub fn parse_url_query_string<'a>(query: &'a str, search_key: &str) -> Option<&'a str> {
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
