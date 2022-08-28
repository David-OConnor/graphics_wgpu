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

use std::sync::atomic::AtomicUsize;

use wgpu::{self, util::DeviceExt, BindGroup, BindGroupLayout, Surface, SurfaceConfiguration};

use crate::{
    // texture,
    camera::Camera,
    input,
    lin_alg::{Quaternion, Vec3},
    types::{Brush, Entity, Instance, Mesh, Scene, Vertex},
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

    // Indices are arranged CCW, from front of face
    #[rustfmt::skip]
        let indices: &[u16] = &[
        0, 2, 1,
        0, 1, 3,
        0, 3, 2,
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
    instances: Vec<Instance>,
    instance_buffer: wgpu::Buffer,
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
            label: Some("Vertex buffer"),
            contents: &vertex_buf,
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index buffer"),
            contents: &index_buf,
            usage: wgpu::BufferUsages::INDEX,
        });

        let instances = (0..6)
            .flat_map(|z| {
                (0..NUM_INSTANCES_PER_ROW).map(move |x| {
                    let position = Vec3::new(x, 0., z) - 5.;

                    let rotation = Quaternion::new_identity();

                    Instance {
                        position,
                        rotation,
                        scale: 1.,
                    }
                })
            })
            .collect::<Vec<_>>();

        let instance_data = instances.iter().map(Instance::to_bytes).collect::<Vec<_>>();
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance buffer"),
            contents: &instance_data,
            usage: wgpu::BufferUsages::VERTEX,
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
            label: Some("Texture"),
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
        camera.update_proj_mat();

        // Create other resources
        let cam_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera buffer"),
            contents: &camera.to_uniform().to_bytes(),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_groups = create_bindgroups(&device, &cam_buf);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let vertex_buffers = [Vertex::desc()];

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline"),
            bind_group_layouts: &[&bind_groups.layout_cam],
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
            camera_buf: cam_buf,
            pipeline,
            // pipeline_wire,
            camera,
            staging_belt: wgpu::util::StagingBelt::new(0x100),
        }
    }

    #[allow(clippy::single_match)]
    pub fn update(&mut self, event: DeviceEvent, queue: &wgpu::Queue) {
        input::handle_event(event, &mut self.camera);

        // todo: ALternative approach that may be more performant:
        // "We can create a separate buffer and copy its contents to our camera_buffer. The new buffer
        // is known as a staging buffer. This method is usually how it's done
        // as it allows the contents of the main buffer (in this case camera_buffer)
        // to only be accessible by the gpu. The gpu can do some speed optimizations which
        // it couldn't if we could access the buffer via the cpu."

        // self.queue.write_buffer(
        queue.write_buffer(&self.camera_buf, 0, &self.camera.to_uniform().to_bytes());
    }

    pub fn render(&mut self, view: &wgpu::TextureView, device: &wgpu::Device, queue: &wgpu::Queue) {
        // We create a CommandEncoder to create the actual commands to send to the
        // gpu. Most modern graphics frameworks expect commands to be stored in a command buffer
        // before being sent to the gpu. The encoder builds a command buffer that we can then
        // send to the gpu.
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Encoder"),
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
                depth_stencil_attachment: None,
            });

            rpass.set_pipeline(&self.pipeline);
            // rpass.set_bind_group(0, &self.bind_groups.diffuse, &[]);
            // todo: Diffuse bind group?

            rpass.set_bind_group(1, &self.bind_groups.cam, &[]);
            rpass.set_vertex_buffer(0, self.vertex_buf.slice(..));
            rpass.set_index_buffer(self.index_buf.slice(..), wgpu::IndexFormat::Uint16);

            rpass.draw_indexed(0..self.num_indices as u32, 0, 0..1);
        }

        queue.submit(Some(encoder.finish()));
    }
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
        label: Some("Render pipeline"),
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
        // If the pipeline will be used with a multiview render pass, this
        // indicates how many array layers the attachments will have.
        multiview: None,
    })
}

struct BindGroupData {
    // pub layout_diffuse: BindGroupLayout,
    // pub diffuse: BindGroup,
    pub layout_cam: BindGroupLayout,
    pub cam: BindGroup,
}

fn create_bindgroups(device: &wgpu::Device, cam_buf: &wgpu::Buffer) -> BindGroupData {
    // We only need vertex, not fragment info in the camera uniform.
    let layout_cam = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
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

    BindGroupData { layout_cam, cam }
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
