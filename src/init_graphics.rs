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

use std::{sync::atomic::AtomicUsize, time::Duration};

use wgpu::{self, util::DeviceExt, BindGroup, BindGroupLayout, SurfaceConfiguration};

use crate::{
    camera::Camera,
    input::{self, InputsCommanded},
    lighting::{Lighting, PointLight},
    texture::Texture,
    types::{Entity, InputSettings, Instance, Mesh, Scene, Vertex},
};

use lin_alg2::f32::{Quaternion, Vec3};

use winit::event::DeviceEvent;

// use egui::Window;
// use egui_winit::{
//     gfx_backends::wgpu_backend::WgpuBackend, window_backends::winit_backend::WinitBackend,
//     BackendSettings, GfxBackend, UserApp, WindowBackend,
// };

static MESH_I: AtomicUsize = AtomicUsize::new(0);

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;
// const IMAGE_SIZE: u32 = 128;

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
    vertex_buf: wgpu::Buffer,
    index_buf: wgpu::Buffer,
    instance_buf: wgpu::Buffer,
    bind_groups: BindGroupData,
    // pub camera: Camera,
    camera_buf: wgpu::Buffer,
    lighting_buf: wgpu::Buffer,
    pipeline: wgpu::RenderPipeline,
    pub depth_texture: Texture,
    pub input_settings: InputSettings,
    inputs_commanded: InputsCommanded,

    // todo: Will this need to change for multiple models
    // obj_mesh: Mesh,
    staging_belt: wgpu::util::StagingBelt, // todo: Do we want this? Probably in sys, not here.
    pub scene: Scene,
    // todo: FIgure out if youw ant this.
    mesh_mappings: Vec<(i32, u32, u32)>,
}

impl GraphicsState {
    pub(crate) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_cfg: &SurfaceConfiguration,
        mut scene: Scene,
        input_settings: InputSettings,
    ) -> Self {
        // GUI code

        // end GUI code test

        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for (i, mesh) in scene.meshes.iter().enumerate() {
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

        let vertex_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex buffer"),
            contents: &vertex_data,
            usage: wgpu::BufferUsages::VERTEX,
        });

        let mut index_data = Vec::new();
        for index in indices {
            let bytes = index.to_ne_bytes();
            index_data.push(bytes[0]);
            index_data.push(bytes[1]);
            index_data.push(bytes[2]);
            index_data.push(bytes[3]);
        }

        let index_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index buffer"),
            contents: &index_data,
            usage: wgpu::BufferUsages::INDEX,
        });

        // let mut camera = Camera::default();
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

        let bind_groups = create_bindgroups(&device, &cam_buf, &lighting_buf);

        let depth_texture = Texture::create_depth_texture(device, surface_cfg, "Depth texture");

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render pipeline layout"),
            bind_group_layouts: &[&bind_groups.layout_cam, &bind_groups.layout_lighting],
            push_constant_ranges: &[],
        });

        let pipeline = create_render_pipeline(device, &pipeline_layout, shader, surface_cfg);

        // We initialize instances, the instance buffer and mesh mappings in `setup_entities`.
        // let instances = Vec::new();
        let instance_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance buffer"),
            contents: &[], // empty on init
            usage: wgpu::BufferUsages::VERTEX,
        });

        // Placeholder value
        let mesh_mappings = Vec::new();

        let mut result = Self {
            vertex_buf,
            index_buf,
            instance_buf,
            bind_groups,
            // camera,
            camera_buf: cam_buf,
            lighting_buf,
            pipeline,
            depth_texture,
            staging_belt: wgpu::util::StagingBelt::new(0x100),
            scene,
            input_settings,
            inputs_commanded: Default::default(),
            mesh_mappings,
        };

        result.setup_entities(&device);

        result
    }

    pub(crate) fn handle_input(&mut self, event: DeviceEvent) {
        input::add_input_cmd(event, &mut self.inputs_commanded);
    }

    /// Currently, sets up entities, but doesn't change meshes, lights, or the camera.
    /// The vertex and index buffers aren't changed; only the instances.
    /// todo: Consider what you want out of this.
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

        // todo: Heper fn that takes a `ToBytes` trait we haven't made?
        let mut instance_data = Vec::new();
        for instance in &instances {
            for byte in instance.to_bytes() {
                instance_data.push(byte);
            }
        }

        let instance_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance buffer"),
            contents: &instance_data,
            usage: wgpu::BufferUsages::VERTEX,
        });

        self.instance_buf = instance_buf;
        self.mesh_mappings = mesh_mappings;
    }

    // #[allow(clippy::single_match)]
    // pub(crate) fn update(&mut self, queue: &wgpu::Queue, dt: Duration) {
    //     // todo: What does this fn do? Probably remove it.
    //
    //     // todo: ALternative approach that may be more performant:
    //     // "We can create a separate buffer and copy its contents to our camera_buffer. The new buffer
    //     // is known as a staging buffer. This method is usually how it's done
    //     // as it allows the contents of the main buffer (in this case camera_buffer)
    //     // to only be accessible by the gpu. The gpu can do some speed optimizations which
    //     // it couldn't if we could access the buffer via the cpu."
    //
    //     let dt_secs = dt.as_secs() as f32 + dt.subsec_micros() as f32 / 1_000_000.;
    //
    //     // input::adjust_camera(
    //     //     &mut self.scene.camera,
    //     //     &self.inputs_commanded,
    //     //     &self.input_settings,
    //     //     dt_secs,
    //     // );
    //
    //     // Reset inputs so they don't stick through the next frame.
    //     self.inputs_commanded = Default::default();
    //
    //     queue.write_buffer(&self.camera_buf, 0, &self.camera.to_bytes());
    // }

    pub(crate) fn render(
        &mut self,
        view: &wgpu::TextureView,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        dt: Duration,
    ) {
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

        // We create a CommandEncoder to create the actual commands to send to the
        // gpu. Most modern graphics frameworks expect commands to be stored in a command buffer
        // before being sent to the gpu. The encoder builds a command buffer that we can then
        // send to the gpu.
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render encoder"),
        });

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
                    view,
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

            rpass.set_pipeline(&self.pipeline);

            rpass.set_bind_group(0, &self.bind_groups.cam, &[]);
            rpass.set_bind_group(1, &self.bind_groups.lighting, &[]);

            rpass.set_vertex_buffer(0, self.vertex_buf.slice(..));
            rpass.set_vertex_buffer(1, self.instance_buf.slice(..));
            rpass.set_index_buffer(self.index_buf.slice(..), wgpu::IndexFormat::Uint32);

            // rpass.set_bind_group(0, &material.bind_group, &[]);

            // rpass.set_bind_group(2, light_bind_group, &[]);
            let mut start_ind = 0; // todo temp?
            for (i, mesh) in self.scene.meshes.iter().enumerate() {
                let (vertex_start_this_mesh, instance_start_this_mesh, instance_count_this_mesh) =
                    self.mesh_mappings[i];

                rpass.draw_indexed(
                    // 0..mesh.indices.len() as u32,
                    start_ind..start_ind + mesh.indices.len() as u32,
                    vertex_start_this_mesh,
                    instance_start_this_mesh..instance_start_this_mesh + instance_count_this_mesh,
                );

                start_ind += mesh.indices.len() as u32; // todo temp?
            }
        }

        // todo: Make sure if you add new instances to the Vec, that you recreate the instance_buffer
        // todo and as well as camera_bind_group, otherwise your new instances won't show up correctly.

        queue.submit(Some(encoder.finish()));
        // queue.submit(iter::once(encoder.finish()));
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

struct BindGroupData {
    pub layout_cam: BindGroupLayout,
    pub cam: BindGroup,
    pub layout_lighting: BindGroupLayout,
    pub lighting: BindGroup,
}

fn create_bindgroups(
    device: &wgpu::Device,
    cam_buf: &wgpu::Buffer,
    lighting_buf: &wgpu::Buffer,
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

    BindGroupData {
        layout_cam,
        cam,
        layout_lighting,
        lighting,
    }
}
