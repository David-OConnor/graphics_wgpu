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

const BG_COLOR: wgpu::Color = wgpu::Color {
    r: 0.7,
    g: 0.7,
    b: 0.7,
    a: 1.0,
};

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
    // meshes: Vec<Mesh>,
    vertex_buf: wgpu::Buffer,
    index_buf: wgpu::Buffer,
    num_indices: usize,
    instances: Vec<Instance>,
    instance_buf: wgpu::Buffer,
    bind_groups: BindGroupData,
    pub camera: Camera,
    camera_buf: wgpu::Buffer,
    // lighting: Lighting,
    lighting_buf: wgpu::Buffer,
    point_lights: Vec<PointLight>,
    point_light_buf: wgpu::Buffer,
    pipeline: wgpu::RenderPipeline,
    // depth_texture: wgpu::Texture,
    pub depth_texture: Texture,
    pub input_settings: InputSettings,
    inputs_commanded: InputsCommanded,

    // todo: Will this need to change for multiple models
    // obj_mesh: Mesh,
    staging_belt: wgpu::util::StagingBelt, // todo: Do we want this? Probably in sys, not here.
    scene: Scene,
}

impl GraphicsState {
    pub(crate) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_cfg: &SurfaceConfiguration,
        scene: Scene,
        input_settings: InputSettings,
    ) -> Self {
        // todo: Temp way of doing this; need a way to support multiple meshes
        // todo and entities
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for mesh in &scene.meshes {
            for vertex in &mesh.vertices {
                vertices.push(vertex)
            }
            for index in &mesh.indices {
                indices.push(index)
            }
        }

        let num_indices = indices.len();

        let mut instances = vec![];
        for entity in &scene.entities {
            instances.push(Instance {
                // todo: eneity into method?
                position: entity.position,
                orientation: entity.orientation,
                scale: entity.scale,
                color: Vec3::new(entity.color.0, entity.color.1, entity.color.2),
            });
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

        // todo: Heper fn that takes a `ToBytes` trait we haven't made?
        let mut instance_data = Vec::new();
        for instance in &instances {
            for byte in instance.to_bytes() {
                instance_data.push(byte);
            }
        }

        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance buffer"),
            contents: &instance_data,
            usage: wgpu::BufferUsages::VERTEX,
        });

        let mut camera = Camera::default();
        camera.update_proj_mat();

        let cam_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera buffer"),
            contents: &camera.to_bytes(),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let lighting_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Lighting buffer"),
            contents: &scene.lighting.to_bytes(),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let mut point_lights = vec![];

        // todo
        let point_light_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Point light buffer"),
            contents: &[],
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_groups = create_bindgroups(&device, &cam_buf, &lighting_buf);
        // let bind_groups = create_bindgroups(&device, &cam_buf, &lighting_buf, &point_light_buf);

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

        // let mut entities = vec![]; // todo!
        // let mut meshes = vec![]; // todo!
        // let mut meshes_wgpu = vec![]; // todo!

        Self {
            // meshes,
            vertex_buf,
            index_buf,
            num_indices,
            instances,
            instance_buf: instance_buffer,
            bind_groups,
            camera,
            camera_buf: cam_buf,
            // lighting,
            lighting_buf,
            point_lights,
            point_light_buf,
            pipeline,
            depth_texture,
            // pipeline_wire,
            staging_belt: wgpu::util::StagingBelt::new(0x100),
            scene,
            input_settings,
            inputs_commanded: Default::default(),
        }
    }

    #[allow(clippy::single_match)]
    pub(crate) fn handle_input(&mut self, event: DeviceEvent) {
        input::add_input_cmd(event, &mut self.inputs_commanded);
    }

    #[allow(clippy::single_match)]
    pub(crate) fn update(&mut self, queue: &wgpu::Queue, dt: Duration) {
        // todo: ALternative approach that may be more performant:
        // "We can create a separate buffer and copy its contents to our camera_buffer. The new buffer
        // is known as a staging buffer. This method is usually how it's done
        // as it allows the contents of the main buffer (in this case camera_buffer)
        // to only be accessible by the gpu. The gpu can do some speed optimizations which
        // it couldn't if we could access the buffer via the cpu."

        let dt_secs = dt.as_secs() as f32 + dt.subsec_micros() as f32 / 1_000_000.;

        input::adjust_camera(
            &mut self.camera,
            &self.inputs_commanded,
            &self.input_settings,
            dt_secs,
        );

        // Reset inputs so they don't stick through the next frame.
        self.inputs_commanded = Default::default();

        queue.write_buffer(&self.camera_buf, 0, &self.camera.to_bytes());
    }

    pub(crate) fn render(
        &mut self,
        view: &wgpu::TextureView,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
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
        //     .copy_from_slice(&self.camera.to_uniform().to_bytes());
        //
        // self.staging_belt.finish();

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(BG_COLOR),
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
            rpass.draw_indexed(
                0..self.num_indices as u32,
                0,
                0..self.instances.len() as u32,
            );

            // mesh.draw_instanced(
            //     &mut rpass,
            //     0..1,
            //     // 0..self.instances.len() as u32,
            //     &self.bind_groups.cam,
            // );
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
            // targets: &[Some(wgpu::ColorTargetState {
            //     format: color_format,
            //     blend: Some(wgpu::BlendState {
            //         alpha: wgpu::BlendComponent::REPLACE,
            //         color: wgpu::BlendComponent::REPLACE,
            //     }),
            //     write_mask: wgpu::ColorWrites::ALL,
            // })],
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
                ty: wgpu::BufferBindingType::Uniform,
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
