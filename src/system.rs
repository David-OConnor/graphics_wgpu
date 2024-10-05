//! This module initiates the window, and graphics hardware.
//! It initializes WGPU settings.

//  Check out this example for winit/egui/wgpu (2024)
// https://github.com/kaphula/winit-egui-wgpu-template/blob/master/src/main.rs

use std::sync::mpsc::{self, Receiver, Sender};
#[cfg(not(target_arch = "wasm32"))]
use std::{
    path::Path,
    time::{Duration, Instant},
};

use image::ImageError;
use wgpu::{
    Adapter, Backends, Features, InstanceDescriptor, PowerPreference, Surface,
    SurfaceConfiguration, TextureFormat,
};
use winit::{
    event::{DeviceEvent, Event, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Icon, Window, WindowAttributes, WindowId},
};

use crate::{
    graphics::GraphicsState,
    texture::Texture,
    types::{EngineUpdates, InputSettings, Scene, UiLayout, UiSettings},
};

// todo: Changed 2024; no idea what this should be
pub const TEXTURE_FORMAT: TextureFormat = TextureFormat::Rgba32Float;

const WINDOW_TITLE_INIT: &str = "Graphics";
const WINDOW_SIZE_X_INIT: f32 = 900.0;
const WINDOW_SIZE_Y_INIT: f32 = 600.0;

pub(crate) struct SystemState {
    pub instance: wgpu::Instance,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub surface: Surface<'static>, // Sshare the same lifetime as the window, A/R.
    pub adapter: Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface_cfg: SurfaceConfiguration,
    /// Used to disable inputs while the mouse is in the GUI section.
    pub mouse_in_gui: bool, // todo: Is this how you want to handle this?
}

pub struct State<T: 'static, FRender, FEvent, FGui>
where
    FRender: FnMut(&mut T, &mut Scene, f32) -> EngineUpdates + 'static,
    FEvent: FnMut(&mut T, DeviceEvent, &mut Scene, f32) -> EngineUpdates + 'static,
    FGui: FnMut(&mut T, &egui::Context, &mut Scene) -> EngineUpdates + 'static,
{
    pub sys: SystemState,
    pub graphics: GraphicsState,
    // Below is part of new Winit system
    // pub windows: HashMap<WindowId, WindowState>,
    pub user_state: T,
    // pub render_handler: impl FnMut(&mut T, &mut Scene, f32) -> EngineUpdates + 'static,
    // pub event_handler: impl FnMut(&mut T, DeviceEvent, &mut Scene, f32) -> EngineUpdates + 'static,
    // pub gui_handler: impl FnMut(&mut T, &egui::Context, &mut Scene) -> EngineUpdates + 'static,
    pub render_handler: FRender,
    pub event_handler: FEvent,
    pub gui_handler: FGui,
    pub last_render_time: Instant,
    pub dt: Duration,
    pub window: Window,
}

impl<T: 'static, FRender, FEvent, FGui> State<T, FRender, FEvent, FGui>
where
    FRender: FnMut(&mut T, &mut Scene, f32) -> EngineUpdates + 'static,
    FEvent: FnMut(&mut T, DeviceEvent, &mut Scene, f32) -> EngineUpdates + 'static,
    FGui: FnMut(&mut T, &egui::Context, &mut Scene) -> EngineUpdates + 'static,
{
    pub(crate) fn new(
        window: Window,
        scene: Scene,
        input_settings: InputSettings,
        ui_settings: UiSettings,
        user_state: T,
        // render_handler: impl FnMut(&mut T, &mut Scene, f32) -> EngineUpdates + 'static,
        // event_handler: impl FnMut(&mut T, DeviceEvent, &mut Scene, f32) -> EngineUpdates + 'static,
        // gui_handler: impl FnMut(&mut T, &egui::Context, &mut Scene) -> EngineUpdates + 'static,
        render_handler: FRender,
        event_handler: FEvent,
        gui_handler: FGui,
    ) -> Self {
        let size = window.inner_size();

        // The instance is a handle to our GPU. Its main purpose is to create Adapters and Surfaces.
        let instance = wgpu::Instance::new(InstanceDescriptor {
            backends: Backends::VULKAN,
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let (adapter, device, queue) = pollster::block_on(setup_async(&instance, &surface));

        // The surface is the part of the window that we draw to. We need it to draw directly to the
        // screen. Our window needs to implement raw-window-handle (opens new window)'s
        // HasRawWindowHandle trait to create a surface.

        // https://docs.rs/wgpu/latest/wgpu/type.SurfaceConfiguration.html
        let surface_cfg = SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            // format: surface.get_supported_formats(&adapter)[0],
            format: TEXTURE_FORMAT,
            width: size.width,
            height: size.height,
            // https://docs.rs/wgpu/latest/wgpu/enum.PresentMode.html
            // Note that `Fifo` locks FPS to the speed of the monitor.
            present_mode: wgpu::PresentMode::Fifo,
            // todo: Allow config from user.
            // present_mode: wgpu::PresentMode::Immediate,
            // present_mode: wgpu::PresentMode::Mailbox,
            desired_maximum_frame_latency: 2, // Default
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: Vec::new(),
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
            mouse_in_gui: false,
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

        let last_render_time = Instant::now();
        let dt = Duration::new(0, 0);

        Self {
            sys,
            graphics,
            user_state,
            render_handler,
            event_handler,
            gui_handler,
            last_render_time,
            dt,
            window,
        }
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

/// This is the entry point to the renderer. It's called by the application to initialize the event
/// loop.
pub fn run<T: 'static, FRender, FEvent, FGui>(
    user_state: T,
    scene: Scene,
    input_settings: InputSettings,
    ui_settings: UiSettings,
    // mut render_handler: impl FnMut(&mut T, &mut Scene, f32) -> EngineUpdates + 'static,
    // mut event_handler: impl FnMut(&mut T, DeviceEvent, &mut Scene, f32) -> EngineUpdates + 'static,
    // mut gui_handler: impl FnMut(&mut T, &egui::Context, &mut Scene) -> EngineUpdates + 'static,
    render_handler: FRender,
    event_handler: FEvent,
    gui_handler: FGui,
) where
    FRender: FnMut(&mut T, &mut Scene, f32) -> EngineUpdates + 'static,
    FEvent: FnMut(&mut T, DeviceEvent, &mut Scene, f32) -> EngineUpdates + 'static,
    FGui: FnMut(&mut T, &egui::Context, &mut Scene) -> EngineUpdates + 'static,
{
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

    let event_loop = EventLoop::new().unwrap();

    let window_attributes = WindowAttributes::default()
        .with_title(WINDOW_TITLE_INIT)
        .with_inner_size(winit::dpi::LogicalSize::new(
            WINDOW_SIZE_X_INIT,
            WINDOW_SIZE_Y_INIT,
        ))
        .with_window_icon(icon);

    let window = event_loop.create_window(window_attributes).unwrap();

    let mut state: State<T, FRender, FEvent, FGui> = State::new(
        window,
        scene,
        input_settings,
        ui_settings,
        user_state,
        render_handler,
        event_handler,
        gui_handler,
    );

    event_loop.run_app(&mut state).unwrap();

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

/// Quarantine for the Async part of the API
async fn setup_async(
    instance: &wgpu::Instance,
    surface: &Surface<'static>,
) -> (wgpu::Adapter, wgpu::Device, wgpu::Queue) {
    // The adapter is a handle to our actual graphics card. You can use this to get
    // information about the graphics card such as its name and what backend the
    // adapter uses. We use this to create our Device and Queue.
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            // `Default` prefers low power when on battery, high performance when on mains.
            power_preference: PowerPreference::default(),
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
                required_features: Features::empty(),
                // https://docs.rs/wgpu/latest/wgpu/struct.Limits.html
                required_limits: Default::default(),
                memory_hints: Default::default(),
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
