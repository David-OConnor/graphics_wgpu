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

use std::{
    mem,
    sync::atomic::{AtomicUsize, Ordering},
};

use wgpu::{self, util::DeviceExt, BindGroup, BindGroupLayout, Surface, SurfaceConfiguration};

use crate::{
    input,
    lin_alg::{Quaternion, Vec3},
    // texture,
    types::{Brush, Camera, Entity, Mesh, Scene, Vertex, CAM_SIZE, VERTEX_SIZE},
};

use winit::event::DeviceEvent;

const BG_COLOR: wgpu::Color = wgpu::Color {
    r: 0.1,
    g: 0.2,
    b: 0.3,
    a: 1.0,
};

static MESH_I: AtomicUsize = AtomicUsize::new(0);

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24Plus;
const IMAGE_SIZE: u32 = 128;

pub const DT: f32 = 1. / 60.; //ie the inverse of frame rate.

pub const UP_VEC: Vec3 = Vec3 {
    x: 0.,
    y: 1.,
    z: 0.,
};
pub const RIGHT_VEC: Vec3 = Vec3 {
    x: 1.,
    y: 0.,
    z: 0.,
};
pub const FWD_VEC: Vec3 = Vec3 {
    x: 0.,
    y: 0.,
    z: 1.,
};

// todo: INstead of this, create a brush, then convert it to a mesh.
// todo: Do this once your renderer works using this hardcoded tetrahedron.
fn create_vertices() -> (Vec<Vertex>, Vec<u16>) {
    // todo: Normals etc on these?
    // This forms a tetrahedron
    let vertices = [
        Vertex::new(1., 1., 1.),
        Vertex::new(1., -1., -1.),
        Vertex::new(-1., 1., -1.),
        Vertex::new(-1., -1., 1.),
    ];

    // These indices define faces by triangles. (each 3 represent a triangle, starting at index 0.
    // todo: You have code in `types` to split a face into triangles for mesh construction.

    #[rustfmt::skip]
        let indices: &[u16] = &[
        0, 1, 2,
        0, 1, 3,
        0, 2, 3,
        1, 2, 3,
    ];

    // todo: Consider imlementing this.
    let faces = vec![
        vec![0, 1, 2], // since each face is a tri, this is the same as indices
        vec![0, 1, 3],
        vec![0, 2, 3],
        vec![1, 2, 3],
    ];
    let brush = Brush::new(vertices.to_vec(), faces);

    (vertices.to_vec(), indices.to_vec())
}

fn create_texels(size: usize) -> Vec<u8> {
    (0..size * size)
        .map(|id| {
            // get high five for recognizing this ;)
            let cx = 3.0 * (id % size) as f32 / (size - 1) as f32 - 2.0;
            let cy = 2.0 * (id / size) as f32 / (size - 1) as f32 - 1.0;
            let (mut x, mut y, mut count) = (cx, cy, 0);
            while count < 0xFF && x * x + y * y < 4.0 {
                let old_x = x;
                x = x * x - y * y + cx;
                y = 2.0 * old_x * y + cy;
                count += 1;
            }
            count
        })
        .collect()
}

pub struct State {
    vertex_buf: wgpu::Buffer,
    index_buf: wgpu::Buffer,
    num_indices: usize,
    bind_groups: BindGroupData,
    camera_buf: wgpu::Buffer,
    pipeline: wgpu::RenderPipeline,
    camera: Camera,
    staging_belt: wgpu::util::StagingBelt, // todo: Do we want this?
}

impl State {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_cfg: &SurfaceConfiguration,
    ) -> Self {
        // Create the vertex and index buffers
        let (vertex_data, index_data) = create_vertices();
        let num_indices = index_data.len();

        // Convert the vertex and index data to u8 buffers.
        let mut vertex_buf = Vec::new();
        for vertex in vertex_data {
            for byte in vertex.to_bytes() {
                vertex_buf.push(byte);
            }
        }

        let mut index_buf = Vec::new();
        for index in index_data {
            let bytes = index.to_le_bytes();
            index_buf.push(bytes[0]);
            index_buf.push(bytes[1]);
        }

        let vertex_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: &vertex_buf,
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: &index_buf,
            usage: wgpu::BufferUsages::INDEX,
        });

        // Create the texture
        let size = 256u32;
        let texels = create_texels(size as usize);
        let texture_extent = wgpu::Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: texture_extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Uint,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        });

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        queue.write_texture(
            texture.as_image_copy(),
            &texels,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(std::num::NonZeroU32::new(size).unwrap()),
                rows_per_image: None,
            },
            texture_extent,
        );

        let mut camera = Camera::default();
        camera.update_proj_mats();

        // Create other resources
        let camera_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: &camera.to_bytes(),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_groups = create_bindgroups(&device, &texture_view, &camera_buf);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let vertex_buffers = [Vertex::desc()];

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_groups.bg_layout],
            push_constant_ranges: &[],
        });

        let pipeline = create_render_pipeline(
            device,
            &pipeline_layout,
            &vertex_buffers,
            shader,
            surface_cfg,
        );

        // let mut entities = vec![]; // todo!
        // let mut meshes = vec![]; // todo!
        // let mut meshes_wgpu = vec![]; // todo!

        Self {
            vertex_buf,
            index_buf,
            num_indices,
            bind_groups,
            camera_buf,
            pipeline,
            // pipeline_wire,
            camera,
            staging_belt: wgpu::util::StagingBelt::new(0x100),
        }
    }

    #[allow(clippy::single_match)]
    pub fn update(&mut self, event: DeviceEvent) {
        input::handle_event(event, &mut self.camera);
    }

    pub fn render(&mut self, view: &wgpu::TextureView, device: &wgpu::Device, queue: &wgpu::Queue) {
        // We create a CommandEncoder to create the actual commands to send to the
        // gpu. Most modern graphics frameworks expect commands to be stored in a command buffer
        // before being sent to the gpu. The encoder builds a command buffer that we can then
        // send to the gpu.
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        self.staging_belt
            .write_buffer(
                &mut encoder,
                &self.camera_buf,
                0,
                // x4 since all value are f32.
                wgpu::BufferSize::new(CAM_SIZE as wgpu::BufferAddress).unwrap(),
                device,
            )
            .copy_from_slice(&self.camera.to_bytes());

        self.staging_belt.finish();

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(BG_COLOR),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            rpass.push_debug_group("Prepare data for draw.");
            rpass.set_pipeline(&self.pipeline);
            rpass.set_bind_group(0, &self.bind_groups.bind_group, &[]);
            rpass.set_index_buffer(self.index_buf.slice(..), wgpu::IndexFormat::Uint16);
            rpass.set_vertex_buffer(0, self.vertex_buf.slice(..));
            // rpass.draw(0..self.num_vertices, 0..1);
            rpass.pop_debug_group();
            rpass.insert_debug_marker("Draw!");
            rpass.draw_indexed(0..self.num_indices as u32, 0, 0..1);
        }

        queue.submit(Some(encoder.finish()));
    }
}

struct BindGroupData {
    pub bg_layout: BindGroupLayout,
    pub bind_group: BindGroup,
}

/// Create render pipelines.
fn create_render_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    vertex_buffers: &[wgpu::VertexBufferLayout],
    shader: wgpu::ShaderModule,
    config: &SurfaceConfiguration,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &vertex_buffers,
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
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    })
}

fn create_bindgroups(
    device: &wgpu::Device,
    texture_view: &wgpu::TextureView,
    uniform_buf: &wgpu::Buffer,
) -> BindGroupData {
    // Create pipeline layout
    let bg_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    sample_type: wgpu::TextureSampleType::Uint,
                    view_dimension: wgpu::TextureViewDimension::D2,
                },
                count: None,
            },
        ],
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &bg_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&texture_view),
            },
        ],
        label: None,
    });

    BindGroupData {
        bg_layout,
        bind_group,
    }
}

fn add_scene_entities(entities: &mut Vec<Entity>) {
    // todo: 2022-08-22: Use this to create your scene.

    let cuboid1 = Brush::make_cuboid(10., 10., 10.);
    // let mesh1 = Mesh::from_brush(cuboid1);
    //
    // let entity1 = Entity {
    //     mesh: MESH_I.fetch_add(1, Ordering::Release),
    //     position: Vec3::new(70., 5., 20.),
    //     orientation: Quaternion::new_identity(),
    //     scale: 1.,
    // };
    //
    // entities.push(entity1);
    // meshes.push(mesh1);
    //
    // let floor_brush = Brush::make_cuboid(100., -1., 100.);
    // let floor_mesh = Mesh::from_brush(floor_brush);
    //
    // let floor_entity = Entity {
    //     mesh: MESH_I.fetch_add(1, Ordering::Release),
    //     position: Vec3::new(0., -0.5, 0.),
    //     orientation: Quaternion::new_identity(),
    //     scale: 1.,
    // };
    //
    // // entities.push(floor_entity);
    // // meshes.push(floor_mesh);
}
