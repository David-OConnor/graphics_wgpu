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

// todo: Remove Cows.

use std::{
    borrow::Cow,
    iter, mem,
    ops::Range,
    sync::atomic::{AtomicUsize, Ordering},
};

use wgpu::{self, util::DeviceExt, BindGroup, BindGroupLayout, Surface, SurfaceConfiguration};

use super::{
    input,
    lin_alg::{Quaternion, Vec3},
    texture,
    types::{Brush, Camera, Entity, Mesh, Scene, Vertex},
    // types_wgpu::{self, CameraUniform, Material, MeshWgpu, Model, VertexWgpu},
    types_wgpu::VertexWgpu,
};

use winit::event::DeviceEvent;

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
    index_count: usize,
    bind_groups: BindGroupData,
    uniform_buf: wgpu::Buffer,
    pipeline: wgpu::RenderPipeline,
    pipeline_wire: Option<wgpu::RenderPipeline>,
    camera: Camera,
    // camera_uniform: CameraUniform,
    staging_belt: wgpu::util::StagingBelt, // todo: Do we want this?
}

impl State {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_cfg: &SurfaceConfiguration,
    ) -> Self {
        // let mut camera_uniform = types_wgpu::CameraUniform::new();
        // camera_uniform.update_view_proj(&camera, &camera.projection_mat);
        //
        // let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        //     label: Some("Camera Buffer"),
        //     contents: bytemuck::cast_slice(&[camera_uniform]),
        //     usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        // });

        // Create the vertex and index buffers
        let vertex_size = mem::size_of::<VertexWgpu>();
        let (vertex_data, index_data) = create_vertices();

        let mut vertex_data_wgpu: Vec<VertexWgpu> = Vec::new();
        for v in vertex_data {
            vertex_data_wgpu.push((&v).into());
        }

        let vertex_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertex_data_wgpu),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&index_data),
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

        let camera_uniform = camera.to_uniform_data();

        // Create other resources

        let uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&camera_uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_groups = create_bindgroups(&device, &texture_view, &uniform_buf);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
        });

        let vertex_buffers = [wgpu::VertexBufferLayout {
            array_stride: vertex_size as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 0,
                    shader_location: 0,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 4 * 4,
                    shader_location: 1,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 4 * 4, // todo?
                    shader_location: 2,
                },
            ],
        }];

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_groups.bg_layout],
            push_constant_ranges: &[],
        });

        let pipeline_wire = if device
            .features()
            .contains(wgpu::Features::POLYGON_MODE_LINE)
        {
            let pipeline_wire = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &vertex_buffers,
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fs_wire",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: surface_cfg.format,
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent {
                                operation: wgpu::BlendOperation::Add,
                                src_factor: wgpu::BlendFactor::SrcAlpha,
                                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            },
                            alpha: wgpu::BlendComponent::REPLACE,
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    polygon_mode: wgpu::PolygonMode::Line,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
            });
            Some(pipeline_wire)
        } else {
            None
        };

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
            index_count: index_data.len(),
            bind_groups,
            uniform_buf,
            pipeline,
            pipeline_wire,
            camera,
            // camera_uniform,
            staging_belt: wgpu::util::StagingBelt::new(0x100),
        }
    }

    #[allow(clippy::single_match)]
    pub fn update(&mut self, event: DeviceEvent) {
        input::handle_event(event, &mut self.camera);
    }

    pub fn render(
        &mut self,
        view: &wgpu::TextureView,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        // spawner: &framework::Spawner,
    ) {
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        // Update the camera buffer:
        let camera_uniform = self.camera.to_uniform_data();

        self.staging_belt
            .write_buffer(
                &mut encoder,
                &self.uniform_buf,
                0,
                // x4 since all value are f32.
                wgpu::BufferSize::new((camera_uniform.len() * 4) as wgpu::BufferAddress).unwrap(),
                device,
            )
            .copy_from_slice(bytemuck::cast_slice(&camera_uniform));

        self.staging_belt.finish();

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
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
            rpass.pop_debug_group();
            rpass.insert_debug_marker("Draw!");
            rpass.draw_indexed(0..self.index_count as u32, 0, 0..1);

            if let Some(ref pipe) = self.pipeline_wire {
                rpass.set_pipeline(pipe);
                rpass.draw_indexed(0..self.index_count as u32, 0, 0..1);
            }
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
    // color_format: wgpu::TextureFormat,
    // depth_format: Option<wgpu::TextureFormat>,
    // vertex_layouts: &[wgpu::VertexBufferLayout],
    // shader: wgpu::ShaderModuleDescriptor,
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
            cull_mode: Some(wgpu::Face::Back),
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    })
}

fn create_bindgroups(
    device: &wgpu::Device,
    // camera_buffer: &wgpu::Buffer,
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
