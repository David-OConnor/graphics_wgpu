//! This module contains code specific to the WGPU library.
//! See [Official WGPU examples](https://github.com/gfx-rs/wgpu/tree/master/wgpu/examples)
//! See [Bevy Garphics](https://github.com/bevyengine/bevy/blob/main/crates/bevy_render) for
//! a full graphics engine example that uses Wgpu.
//! https://sotrh.github.io/learn-wgpu/
//!
//! https://github.com/sotrh/learn-wgpu/tree/master/code/intermediate/tutorial12-camera/src
//! https://github.com/gfx-rs/wgpu/tree/master/wgpu/examples/shadow
//!
//! 2022-08-21: https://github.com/gfx-rs/wgpu/blob/master/wgpu/examples/cube/main.rs

use std::{num::NonZeroU32, time::Duration};

use wgpu::{self, util::DeviceExt, BindGroup, BindGroupLayout, SurfaceConfiguration};

use crate::{
    compute, gui,
    input::{self, InputsCommanded},
    texture::Texture,
    types::{
        ControlScheme, EngineUpdates, InputSettings, Instance, Scene, UiLayout, UiSettings, Vertex,
    },
};
use lin_alg2::f32::Vec3;

use winit::{event::DeviceEvent, window::Window};

use egui_wgpu_backend::{RenderPass, ScreenDescriptor};
use egui_winit_platform::Platform;

pub(crate) const UP_VEC: Vec3 = Vec3 {
    x: 0.,
    y: 1.,
    z: 0.,
};
pub(crate) const RIGHT_VEC: Vec3 = Vec3 {
    x: 1.,
    y: 0.,
    z: 0.,
};
pub(crate) const FWD_VEC: Vec3 = Vec3 {
    x: 0.,
    y: 0.,
    z: 1.,
};

pub(crate) struct GraphicsState {
    pub vertex_buf: wgpu::Buffer,
    pub index_buf: wgpu::Buffer,
    instance_buf: wgpu::Buffer,
    compute_staging_buf: wgpu::Buffer,
    compute_storage_buf_input: wgpu::Buffer,
    compute_storage_buf_output: wgpu::Buffer,
    pub bind_groups: BindGroupData,
    camera_buf: wgpu::Buffer,
    lighting_buf: wgpu::Buffer,
    pub pipeline_graphics: wgpu::RenderPipeline,
    pipeline_compute: wgpu::ComputePipeline,
    pub depth_texture: Texture,
    pub input_settings: InputSettings,
    pub ui_settings: UiSettings,
    pub inputs_commanded: InputsCommanded,
    // staging_belt: wgpu::util::StagingBelt, // todo: Do we want this? Probably in sys, not here.
    pub scene: Scene,
    mesh_mappings: Vec<(i32, u32, u32)>,
    /// for GUI
    pub egui_platform: Platform,
    pub rpass_egui: RenderPass,
}

impl GraphicsState {
    pub(crate) fn new(
        device: &wgpu::Device,
        // queue: &wgpu::Queue,
        surface_cfg: &SurfaceConfiguration,
        mut scene: Scene,
        input_settings: InputSettings,
        ui_settings: UiSettings,
        // these 3 args are for EGUI
        window: &Window,
        // adapter: &wgpu::Adapter,
        compute_shader: &str, // Shader file as UTF-8
    ) -> Self {
        let vertex_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex buffer"),
            contents: &[], // Populated later.
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index buffer"),
            contents: &[], // Populated later.
            usage: wgpu::BufferUsages::INDEX,
        });

        scene.camera.update_proj_mat();

        let cam_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera buffer"),
            contents: &scene.camera.to_bytes(),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let lighting_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Lighting buffer"),
            contents: &scene.lighting.to_bytes(),
            // We use a storage buffer, since our lighting size is unknown by the shader;
            // this is due to the dynamic-sized point light array.
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        //

        let (compute_storage_buf_input, compute_storage_buf_output, compute_staging_buf) =
            compute::setup(device);

        let bind_groups = create_bindgroups(
            device,
            &cam_buf,
            &lighting_buf,
            &compute_storage_buf_input,
            &compute_storage_buf_output,
        );

        let depth_texture = Texture::create_depth_texture(device, surface_cfg, "Depth texture");

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Graphics shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        // todo: Pass the shader file as a parameter.
        let shader_compute = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Compute shader"),
            source: wgpu::ShaderSource::Wgsl(compute_shader.into()),
        });

        let pipeline_layout_graphics =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render pipeline layout"),
                bind_group_layouts: &[&bind_groups.layout_cam, &bind_groups.layout_lighting],
                push_constant_ranges: &[],
            });

        let pipeline_graphics =
            create_render_pipeline(device, &pipeline_layout_graphics, shader, surface_cfg);

        // todo compute pipeline layout? Not in example
        let pipeline_layout_compute =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Compute pipeline layout"),
                bind_group_layouts: &[&bind_groups.layout_compute],
                push_constant_ranges: &[],
            });

        let pipeline_compute = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute pipeline"),
            layout: Some(&pipeline_layout_compute),
            module: &shader_compute,
            entry_point: "main",
        });

        // We initialize instances, the instance buffer and mesh mappings in `setup_entities`.
        // let instances = Vec::new();
        let instance_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance buffer"),
            contents: &[], // empty on init
            usage: wgpu::BufferUsages::VERTEX,
        });

        // Placeholder value
        let mesh_mappings = Vec::new();

        let egui_platform = gui::setup_platform(surface_cfg, window);

        // todo: Logical (scaling by device?) vs physical pixels
        let window_size = winit::dpi::LogicalSize::new(scene.window_size.0, scene.window_size.1);

        window.set_inner_size(window_size);
        window.set_title(&scene.window_title);

        let rpass_egui = RenderPass::new(device, surface_cfg.format, 1);

        // Display the demo application that ships with egui.
        // let mut egui_app = egui_demo_lib::DemoWindows::default();

        let mut result = Self {
            vertex_buf,
            index_buf,
            instance_buf,
            compute_storage_buf_input,
            compute_storage_buf_output,
            compute_staging_buf,
            bind_groups,
            camera_buf: cam_buf,
            lighting_buf,
            pipeline_graphics: pipeline_graphics,
            pipeline_compute,
            depth_texture,
            // staging_belt: wgpu::util::StagingBelt::new(0x100),
            scene,
            input_settings,
            ui_settings,
            inputs_commanded: Default::default(),
            mesh_mappings,
            egui_platform,
            rpass_egui,
            // egui_app,
        };

        result.setup_vertices_indices(device);
        result.setup_entities(device);

        result
    }

    pub(crate) fn handle_input(&mut self, event: DeviceEvent) {
        match self.input_settings.initial_controls {
            ControlScheme::FreeCamera => input::add_input_cmd(event, &mut self.inputs_commanded),
            _ => (),
        }
    }

    /// todo: WIP to update meshes.
    pub(crate) fn setup_vertices_indices(&mut self, device: &wgpu::Device) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for mesh in &self.scene.meshes {
            for vertex in &mesh.vertices {
                vertices.push(vertex)
            }

            for index in &mesh.indices {
                indices.push(index);
            }
        }
        // Convert the vertex and index data to u8 buffers.
        let mut vertex_data = Vec::new();
        for vertex in vertices {
            for byte in vertex.to_bytes() {
                vertex_data.push(byte);
            }
        }

        let mut index_data = Vec::new();
        for index in indices {
            let bytes = index.to_ne_bytes();
            index_data.push(bytes[0]);
            index_data.push(bytes[1]);
            index_data.push(bytes[2]);
            index_data.push(bytes[3]);
        }

        // We can't update using a queue due to buffer size mismatches.
        let vertex_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex buffer"),
            contents: &vertex_data,
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index buffer"),
            contents: &index_data,
            usage: wgpu::BufferUsages::INDEX,
        });

        self.vertex_buf = vertex_buf;
        self.index_buf = index_buf;
    }

    /// Currently, sets up entities (And the associated instance buf), but doesn't change
    /// meshes, lights, or the camera. The vertex and index buffers aren't changed; only the instances.
    pub(crate) fn setup_entities(&mut self, device: &wgpu::Device) {
        let mut instances = Vec::new();

        let mut mesh_mappings = Vec::new();

        let mut vertex_start_this_mesh = 0;
        let mut instance_start_this_mesh = 0;

        for (i, mesh) in self.scene.meshes.iter().enumerate() {
            let mut instance_count_this_mesh = 0;
            for entity in self.scene.entities.iter().filter(|e| e.mesh == i) {
                instances.push(Instance {
                    // todo: entity into method?
                    position: entity.position,
                    orientation: entity.orientation,
                    scale: entity.scale,
                    color: Vec3::new(entity.color.0, entity.color.1, entity.color.2),
                    shinyness: entity.shinyness,
                });
                instance_count_this_mesh += 1;
            }

            mesh_mappings.push((
                vertex_start_this_mesh,
                instance_start_this_mesh,
                instance_count_this_mesh,
            ));

            vertex_start_this_mesh += mesh.vertices.len() as i32;
            instance_start_this_mesh += instance_count_this_mesh;
        }

        // todo: Helper fn that takes a `ToBytes` trait we haven't made?
        let mut instance_data = Vec::new();
        for instance in &instances {
            for byte in instance.to_bytes() {
                instance_data.push(byte);
            }
        }

        // We can't update using a queue due to buffer size mismatches.
        let instance_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance buffer"),
            contents: &instance_data,
            usage: wgpu::BufferUsages::VERTEX,
        });

        self.instance_buf = instance_buf;
        self.mesh_mappings = mesh_mappings;
    }

    pub(crate) fn update_camera(&mut self, queue: &wgpu::Queue) {
        queue.write_buffer(&self.camera_buf, 0, &self.scene.camera.to_bytes());
    }

    pub(crate) fn update_lighting(&mut self, queue: &wgpu::Queue) {
        queue.write_buffer(&self.lighting_buf, 0, &self.scene.lighting.to_bytes());
    }

    pub(crate) fn render<T>(
        &mut self,
        output_frame: wgpu::SurfaceTexture,
        output_view: &wgpu::TextureView,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        dt: Duration,
        width: u32,
        height: u32,
        // surface: &wgpu::Surface,
        window: &Window,
        mut gui_handler: impl FnMut(&mut T, &egui::Context, &mut Scene) -> EngineUpdates,
        user_state: &mut T,
    ) {
        // Adjust camera inputs using the in-engine control scheme.
        // Note that camera settings adjusted by the application code are handled in
        // `update_camera`.
        match self.input_settings.initial_controls {
            ControlScheme::FreeCamera => {
                if self.inputs_commanded.inputs_present() {
                    let dt_secs = dt.as_secs() as f32 + dt.subsec_micros() as f32 / 1_000_000.;
                    input::adjust_camera(
                        &mut self.scene.camera,
                        &self.inputs_commanded,
                        &self.input_settings,
                        dt_secs,
                    );

                    queue.write_buffer(&self.camera_buf, 0, &self.scene.camera.to_bytes());

                    // Reset the mouse inputs; keyboard inputs are reset by their release event.
                    self.inputs_commanded.mouse_delta_x = 0.;
                    self.inputs_commanded.mouse_delta_y = 0.;
                }
            }
            _ => (),
        }

        // We create a CommandEncoder to create the actual commands to send to the
        // gpu. Most modern graphics frameworks expect commands to be stored in a command buffer
        // before being sent to the gpu. The encoder builds a command buffer that we can then
        // send to the gpu.
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render encoder"),
        });
        //
        // // todo: Make sure if you add new instances to the Vec, that you recreate the instance_buffer
        // // todo and as well as camera_bind_group, otherwise your new instances won't show up correctly.
        //
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute pass"),
            });
            cpass.set_pipeline(&self.pipeline_compute);
            cpass.set_bind_group(0, &self.bind_groups.compute, &[]);
            cpass.insert_debug_marker("Compute test 1.");

            // todo: How does this work?
            // Number of cells to run, the (x,y,z) size of item being processed

            // todo: work_group_count as first var to dispatch_workgroups??
            //         let work_group_count =
            // ((NUM_PARTICLES as f32) / (PARTICLES_PER_GROUP as f32)).ceil() as u32;
            let work_group_count = 64; // todo?
            cpass.dispatch_workgroups(work_group_count, 1, 1);
        }

        let compute_size = 8 * 10; // todo: Sync this with buf

        // Sets adds copy operation to command encoder.
        // Will copy data from storage buffer on GPU to staging buffer on CPU.
        encoder.copy_buffer_to_buffer(
            &self.compute_storage_buf_output,
            0,
            &self.compute_staging_buf,
            0,
            compute_size,
        );

        let compute_result = compute::buf_to_vec(&self.compute_staging_buf, device);

        let mut result_vals = Vec::new();

        let mut i = 0;
        for _ in 0..10 {
            result_vals.push(
                f32::from_ne_bytes(compute_result[i..i + 4].try_into().unwrap())
            );
            i += 4;
        }

        println!("Vals: {:?}\n", result_vals);

        // self.staging_belt
        //     .write_buffer(
        //         &mut encoder,
        //         &self.camera_buf,
        //         1, // todo: What should this be?
        //         // x4 since all value are f32.
        //         wgpu::BufferSize::new(CAM_UNIFORM_SIZE as wgpu::BufferAddress).unwrap(),
        //         device,
        //     )
        //     .copy_from_slice(&self.scene.camera.to_uniform().to_bytes());
        //
        // self.staging_belt.finish();

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: output_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: self.scene.background_color.0 as f64,
                            g: self.scene.background_color.1 as f64,
                            b: self.scene.background_color.2 as f64,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            let ui_size = self.ui_settings.size as f32;

            let (x, y, eff_width, eff_height) = match self.ui_settings.layout {
                UiLayout::Left => (ui_size, 0., width as f32 - ui_size, height as f32),
                UiLayout::Right => (0., 0., width as f32 - ui_size, height as f32),
                UiLayout::Top => (0., ui_size, width as f32, height as f32 - ui_size),
                UiLayout::Bottom => (0., 0., width as f32, height as f32 - ui_size),
            };

            // Adjust the portion of the 3D rendering to take up the space not taken up by the UI.
            rpass.set_viewport(x, y, eff_width, eff_height, 0., 1.);

            rpass.set_pipeline(&self.pipeline_graphics);

            rpass.set_bind_group(0, &self.bind_groups.cam, &[]);
            rpass.set_bind_group(1, &self.bind_groups.lighting, &[]);

            rpass.set_vertex_buffer(0, self.vertex_buf.slice(..));
            rpass.set_vertex_buffer(1, self.instance_buf.slice(..));
            rpass.set_index_buffer(self.index_buf.slice(..), wgpu::IndexFormat::Uint32);

            let mut start_ind = 0;
            for (i, mesh) in self.scene.meshes.iter().enumerate() {
                let (vertex_start_this_mesh, instance_start_this_mesh, instance_count_this_mesh) =
                    self.mesh_mappings[i];

                rpass.draw_indexed(
                    start_ind..start_ind + mesh.indices.len() as u32,
                    vertex_start_this_mesh,
                    instance_start_this_mesh..instance_start_this_mesh + instance_count_this_mesh,
                );

                start_ind += mesh.indices.len() as u32;
            }
        }

        // Set up the GUI render.
        let tdelta = gui::render(
            self,
            device,
            queue,
            &mut encoder,
            user_state,
            gui_handler,
            output_view,
            window,
            width,
            height,
        );

        queue.submit(Some(encoder.finish()));
        // queue.submit(iter::once(encoder.finish()));

        // Redraw egui
        output_frame.present();

        self.rpass_egui
            .remove_textures(tdelta)
            .expect("remove texture ok");
    }

    // todo: Testing separating compute from render

    pub fn compute(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render encoder"),
        });

        // todo: Make sure if you add new instances to the Vec, that you recreate the instance_buffer
        // todo and as well as camera_bind_group, otherwise your new instances won't show up correctly.

        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute pass"),
            });
            cpass.set_pipeline(&self.pipeline_compute);
            cpass.set_bind_group(0, &self.bind_groups.compute, &[]);
            cpass.insert_debug_marker("Compute test 1.");

            // todo: How does this work?
            // Number of cells to run, the (x,y,z) size of item being processed

            // todo: work_group_count as first var to dispatch_workgroups??
            //         let work_group_count =
            // ((NUM_PARTICLES as f32) / (PARTICLES_PER_GROUP as f32)).ceil() as u32;
            let work_group_count = 64; // todo?
            cpass.dispatch_workgroups(work_group_count, 1, 1);
        }

        let compute_size = 8 * 10; // todo: Sync this with buf

        // Sets adds copy operation to command encoder.
        // Will copy data from storage buffer on GPU to staging buffer on CPU.
        encoder.copy_buffer_to_buffer(
            &self.compute_storage_buf_output,
            0,
            &self.compute_staging_buf,
            0,
            compute_size,
        );

        let compute_result = compute::buf_to_vec(&self.compute_staging_buf, device);

        let mut result_vals = Vec::new();

        let mut i = 0;
        for _ in 0..10 {
            result_vals.push(f32::from_ne_bytes(
                compute_result[i..i + 4].try_into().unwrap(),
            ));
            i += 4;
        }

        println!("Vals: {:?}\n", result_vals);

        queue.submit(Some(encoder.finish()));
    }
}

/// Create render pipelines.
fn create_render_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: wgpu::ShaderModule,
    config: &SurfaceConfiguration,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render pipeline"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[Vertex::desc(), Instance::desc()],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(config.format.into())],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },

        depth_stencil: Some(wgpu::DepthStencilState {
            format: Texture::DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        // If the pipeline will be used with a multiview render pass, this
        // indicates how many array layers the attachments will have.
        multiview: None,
    })
}

pub(crate) struct BindGroupData {
    pub layout_cam: BindGroupLayout,
    pub cam: BindGroup,
    pub layout_lighting: BindGroupLayout,
    pub lighting: BindGroup,
    /// We use this for GUI.
    pub layout_texture: BindGroupLayout,
    // pub texture: BindGroup,
    pub layout_compute: BindGroupLayout,
    pub compute: BindGroup,
}

fn create_bindgroups(
    device: &wgpu::Device,
    cam_buf: &wgpu::Buffer,
    lighting_buf: &wgpu::Buffer,
    compute_storage_buf_input: &wgpu::Buffer,
    compute_storage_buf_output: &wgpu::Buffer,
) -> BindGroupData {
    // We only need vertex, not fragment info in the camera uniform.
    let layout_cam = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                // The dynamic field indicates whether this buffer will change size or
                // not. This is useful if we want to store an array of things in our uniforms.
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
        label: Some("Camera bind group layout"),
    });

    let cam = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &layout_cam,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: cam_buf.as_entire_binding(),
        }],
        label: Some("Camera bind group"),
    });

    let layout_lighting = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true }, // todo read-only?
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
        label: Some("Lighting bind group layout"),
    });

    let lighting = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &layout_lighting,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: lighting_buf.as_entire_binding(),
        }],
        label: Some("Lighting bind group"),
    });

    // todo: Don't create these (diffuse tex view, sampler every time. Pass as args.
    // We don't need to configure the texture view much, so let's
    // let wgpu define it.
    // let diffuse_bytes = include_bytes!("happy-tree.png");
    // let diffuse_bytes = [];
    // let diffuse_texture = wgpu::texture::Texture::from_bytes(&device, &queue, diffuse_bytes, "happy-tree.png").unwrap();
    //
    // let diffuse_texture_view = diffuse_texture.create_view(&wgpu::TextureViewDescriptor::default());
    // let diffuse_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
    //     address_mode_u: wgpu::AddressMode::ClampToEdge,
    //     address_mode_v: wgpu::AddressMode::ClampToEdge,
    //     address_mode_w: wgpu::AddressMode::ClampToEdge,
    //     mag_filter: wgpu::FilterMode::Linear,
    //     min_filter: wgpu::FilterMode::Nearest,
    //     mipmap_filter: wgpu::FilterMode::Nearest,
    //     ..Default::default()
    // });

    let layout_texture = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("egui_texture_bind_group_layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                // This should match the filterable field of the
                // corresponding Texture entry above.
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });

    // let texture = device.create_bind_group(
    //     &wgpu::BindGroupDescriptor {
    //         layout: &layout_texture,
    //         entries: &[
    //             wgpu::BindGroupEntry {
    //                 binding: 0,
    //                 resource: wgpu::BindingResource::TextureView(&diffuse_texture_view),
    //                 // resource: wgpu::BindingResource::TextureView(&[]), // todo?
    //             },
    //             wgpu::BindGroupEntry {
    //                 binding: 1,
    //                 resource: wgpu::BindingResource::Sampler(&diffuse_sampler),
    //             }
    //         ],
    //         label: Some("Texture bind group"),
    //     });

    // todo: Consider calling `compute::create_bindgroups` separately, vice from this fn.

    let (layout_compute, compute) = compute::create_bindgroups(
        device,
        compute_storage_buf_input,
        compute_storage_buf_output,
    );

    BindGroupData {
        layout_cam,
        cam,
        layout_lighting,
        lighting,
        layout_texture,
        // texture
        layout_compute,
        compute,
    }
}
