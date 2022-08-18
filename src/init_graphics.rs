//! This module contains code specific to the WGPU library.
//! See [Official WGPU examples](https://github.com/gfx-rs/wgpu/tree/master/wgpu/examples)
//! See [Bevy Garphics](https://github.com/bevyengine/bevy/blob/main/crates/bevy_render) for
//! a full graphics engine example that uses Wgpu.
//! https://sotrh.github.io/learn-wgpu/
//!
//! https://github.com/sotrh/learn-wgpu/tree/master/code/intermediate/tutorial12-camera/src
//! https://github.com/gfx-rs/wgpu/tree/master/wgpu/examples/shadow

// todo: Remove Cows.

use std::{
    iter,
    ops::Range,
    sync::atomic::{AtomicUsize, Ordering},
};

use wgpu::{self, util::DeviceExt, BindGroup, BindGroupLayout, Surface, SurfaceConfiguration};

use super::{
    input,
    lin_alg::{Quaternion, Vec3},
    texture,
    types::{Brush, Camera, Entity, Mesh, Scene, Vertex},
    types_wgpu::{self, CameraUniform, Instance, Material, MeshWgpu, Model},
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

pub struct GameState {
    // Buffers
    uniform_buf: wgpu::Buffer,
    light_buffer: wgpu::Buffer,
    light_uniform: types_wgpu::LightUniform,
    bind_groups: BindGroupData,
    // Pipelines
    render_pipeline: wgpu::RenderPipeline,
    light_render_pipeline: wgpu::RenderPipeline,
    obj_model: Model,

    // Misc
    camera: Camera,
    camera_uniform: CameraUniform,
    // depth_view: wgpu::TextureView,
    instance_buffer: wgpu::Buffer,
    depth_texture: texture::Texture,
    // staging_belt: wgpu::util::StagingBelt,
    /// Movement, camera rotation, zoom.
    // todo: Split out control and game-state into a separate struct?
    // Game state
    active_scene: Scene,
    entities: Vec<Entity>,
    meshes: Vec<Mesh>,
    meshes_wgpu: Vec<MeshWgpu>,
    // todo: Experimenting with light
}

impl GameState {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface_cfg: &SurfaceConfiguration,
    ) -> Self {
        // let obj_model =
        //     resources::load_model("cube.obj", &device, &queue, &texture_bind_group_layout)
        //         .await
        //         .unwrap();
        let obj_model = Model {
            meshes: Vec::new(),
            materials: Vec::new(),
        };

        let light_uniform = types_wgpu::LightUniform {
            position: [2.0, 2.0, 2.0],
            _padding: 0,
            color: [1.0, 1.0, 1.0],
            _padding2: 0,
        };

        let light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light VB"),
            contents: bytemuck::cast_slice(&[light_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let mut camera = Camera::default();
        camera.update_proj_mats();

        // let projection = camera.update_proj_mats()
        //     camera::Projection::new(config.width, config.height, cgmath::Deg(45.0), 0.1, 100.0);
        // let camera_controller = camera::CameraController::new(4.0, 0.4);

        let mut camera_uniform = types_wgpu::CameraUniform::new();
        camera_uniform.update_view_proj(&camera, &camera.projection_mat);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let instances = vec![Instance {
            position: Vec3::new(0., 0., 0.),
            orientation: Quaternion::new_identity(),
        }];

        let instance_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform buffer"),
            contents: bytemuck::cast_slice(&camera.to_uniform_data()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let depth_texture =
            texture::Texture::create_depth_texture(&device, &surface_cfg, "depth_texture");

        // let bind_groups = create_bindgroups(device, &sampler, &uniform_buf);
        let bind_groups = create_bindgroups(device, &light_buffer, &camera_buffer);

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &bind_groups.texture_bg_layout,
                    &bind_groups.camera_bg_layout,
                    &bind_groups.light_bg_layout,
                ],
                push_constant_ranges: &[],
            });

        let render_pipeline = {
            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("Normal Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
            };
            create_render_pipeline(
                &device,
                &render_pipeline_layout,
                surface_cfg.format,
                Some(texture::Texture::DEPTH_FORMAT),
                &[
                    types_wgpu::vertex_desc_model(),
                    types_wgpu::vertex_desc_instance(),
                ],
                shader,
            )
        };

        let light_render_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Light Pipeline Layout"),
                bind_group_layouts: &[&bind_groups.camera_bg_layout, &bind_groups.light_bg_layout],
                push_constant_ranges: &[],
            });
            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("Light Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("light.wgsl").into()),
            };
            create_render_pipeline(
                &device,
                &layout,
                surface_cfg.format,
                Some(texture::Texture::DEPTH_FORMAT),
                &[types_wgpu::vertex_desc_model()],
                shader,
            )
        };

        let mut entities = vec![]; // todo!
        let mut meshes = vec![]; // todo!
        let mut meshes_wgpu = vec![]; // todo!

        Self {
            camera,
            render_pipeline,
            light_render_pipeline,
            bind_groups,
            uniform_buf,
            light_uniform,
            light_buffer,
            // depth_view,
            // staging_belt: wgpu::util::StagingBelt::new(0x100),
            active_scene: Scene::default(),
            entities,
            meshes,
            meshes_wgpu,
            //,
            camera_uniform,
            depth_texture,
            instance_buffer,
            obj_model,
        }
    }

    #[allow(clippy::single_match)]
    pub fn update(&mut self, event: DeviceEvent) {
        input::handle_event(event, &mut self.camera);
    }

    // pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
    //     if new_size.width > 0 && new_size.height > 0 {
    //         self.projection.resize(new_size.width, new_size.height);
    //         self.size = new_size;
    //         self.config.width = new_size.width;
    //         self.config.height = new_size.height;
    //         self.surface.configure(&self.device, &self.config);
    //         self.depth_texture =
    //             texture::Texture::create_depth_texture(&self.device, &self.config, "depth_texture");
    //     }
    // }

    pub fn render(
        &mut self,
        view: &wgpu::TextureView,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surface: &Surface,
    ) {
        let output = surface.get_current_texture().unwrap();
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
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
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            render_pass.set_pipeline(&self.light_render_pipeline);
            render_pass.draw_light_model(
                &self.obj_model,
                &self.bind_groups.camera_bg,
                &self.bind_groups.light_bg,
            );

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.draw_model_instanced(
                &self.obj_model,
                0..self.entities.len() as u32,
                &self.bind_groups.camera_bg,
                &self.bind_groups.light_bg,
            );
        }
        queue.submit(iter::once(encoder.finish()));
        output.present();
    }
}

struct BindGroupData {
    pub texture_bg_layout: BindGroupLayout,
    // pub diffuse_bg_layout: BindGroupLayout,
    // pub entity_bg_layout: BindGroupLayout,
    pub light_bg_layout: BindGroupLayout,
    pub camera_bg_layout: BindGroupLayout,
    // pub entity_uniform_bg_layout: BindGroupLayout,
    // pub entity_texture_bg_layout: BindGroupLayout,
    // pub texture_bg: BindGroup,
    // pub diffuse_bg_layout: BindGroupLayout,
    // pub entity_bg: BindGroup,
    // pub entity_uniform_bg: BindGroup,
    // pub entity_texture_bg: BindGroup,
    pub light_bg: BindGroup,
    pub camera_bg: BindGroup,
}

/// Load a model from file
fn load_model(model_source: &[u8], entities: &mut Vec<Entity>, meshes: &mut Vec<Mesh>) {
    let model_data = obj::ObjData::load_buf(&model_source[..]).unwrap();

    for object in model_data.objects {
        for group in object.groups {
            let mut vertices = Vec::new();

            for poly in group.polys {
                for end_index in 2..poly.0.len() {
                    for &index in &[0, end_index - 1, end_index] {
                        let obj::IndexTuple(position_id, _texture_id, normal_id) = poly.0[index];

                        vertices.push(Vertex {
                            position: model_data.position[position_id].into(),
                            normal: model_data.normal[normal_id.unwrap()],
                            tex_coords: [0., 0.],
                            tangent: [0., 0., 0.],
                            bitangent: [0., 0., 0.],
                        })
                    }
                }
            }

            let indices = Vec::new(); // todo temp.

            meshes.push(Mesh { vertices, indices });

            entities.push(Entity {
                mesh: MESH_I.fetch_add(1, Ordering::Relaxed),
                position: Vec3::new(0., 0., 0.),
                rotation: Quaternion::new_identity(),
                scale: 1.,
            });
        }
    }
}

/// Create render pipelines.
fn create_render_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    color_format: wgpu::TextureFormat,
    depth_format: Option<wgpu::TextureFormat>,
    vertex_layouts: &[wgpu::VertexBufferLayout],
    shader: wgpu::ShaderModuleDescriptor,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(shader);

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(&format!("{:?}", shader)),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: vertex_layouts,
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: color_format,
                blend: Some(wgpu::BlendState {
                    alpha: wgpu::BlendComponent::REPLACE,
                    color: wgpu::BlendComponent::REPLACE,
                }),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
            polygon_mode: wgpu::PolygonMode::Fill,
            // Requires Features::DEPTH_CLIP_CONTROL
            unclipped_depth: false,
            // Requires Features::CONSERVATIVE_RASTERIZATION
            conservative: false,
        },
        depth_stencil: depth_format.map(|format| wgpu::DepthStencilState {
            format,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        // If the pipeline will be used with a multiview render pass, this
        // indicates how many array layers the attachments will have.
        multiview: None,
    })
}

fn create_bindgroups(
    device: &wgpu::Device,
    light_buffer: &wgpu::Buffer,
    camera_buffer: &wgpu::Buffer,
    // texture_view: &TextureView,
    // sampler: &Sampler,
    // uniform_buf: &wgpu::Buffer,
) -> BindGroupData {
    // A BindGroup describes a set of resources and how they can be accessed by a shader.
    let texture_bg_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            // normal map
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
        label: Some("texture_bind_group_layout"),
    });
    //
    // let entity_bg_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    //     label: None,
    //     entries: &[
    //         wgpu::BindGroupLayoutEntry {
    //             binding: 0,
    //             visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
    //             ty: wgpu::BindingType::Buffer {
    //                 ty: wgpu::BufferBindingType::Uniform,
    //                 has_dynamic_offset: false,
    //                 min_binding_size: None,
    //             },
    //             count: None,
    //         },
    //         wgpu::BindGroupLayoutEntry {
    //             binding: 1,
    //             visibility: wgpu::ShaderStages::FRAGMENT,
    //             ty: wgpu::BindingType::Texture {
    //                 sample_type: wgpu::TextureSampleType::Float { filterable: true },
    //                 multisampled: false,
    //                 view_dimension: wgpu::TextureViewDimension::Cube,
    //             },
    //             count: None,
    //         },
    //         wgpu::BindGroupLayoutEntry {
    //             binding: 2,
    //             visibility: wgpu::ShaderStages::FRAGMENT,
    //             ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
    //             count: None,
    //         },
    //     ],
    // });
    //
    // let entity_uniform_bg_layout =
    //     device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    //         label: Some("Entity uniform bind group layout"),
    //         entries: &[wgpu::BindGroupLayoutEntry {
    //             binding: 0,
    //             visibility: wgpu::ShaderStages::VERTEX,
    //             ty: wgpu::BindingType::Buffer {
    //                 ty: wgpu::BufferBindingType::Uniform,
    //                 has_dynamic_offset: false,
    //                 min_binding_size: None,
    //             },
    //             count: None,
    //         }],
    //     });
    //
    // let entity_texture_bg_layout =
    //     device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    //         label: Some("Entity texture bind group layout"),
    //         entries: &[
    //             wgpu::BindGroupLayoutEntry {
    //                 binding: 0,
    //                 visibility: wgpu::ShaderStages::FRAGMENT,
    //                 ty: wgpu::BindingType::Texture {
    //                     sample_type: wgpu::TextureSampleType::Float { filterable: true },
    //                     multisampled: false,
    //                     view_dimension: wgpu::TextureViewDimension::D2,
    //                 },
    //                 count: None,
    //             },
    //             wgpu::BindGroupLayoutEntry {
    //                 binding: 1,
    //                 visibility: wgpu::ShaderStages::FRAGMENT,
    //                 ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
    //                 count: None,
    //             },
    //         ],
    //     });
    //

    // let diffuse_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
    //     layout: &texture_bind_group_layout,
    //     entries: &[
    //         wgpu::BindGroupEntry {
    //             binding: 0,
    //             resource: wgpu::BindingResource::TextureView(diffuse_texture_view),
    //         },
    //         wgpu::BindGroupEntry {
    //             binding: 1,
    //             resource: wgpu::BindingResource::Sampler(diffuse_sampler),
    //         },
    //     ],
    //     label: Some("diffuse_bind_group"),
    // });
    //
    // let entity_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
    //     layout: &entity_bg_layout,
    //     entries: &[
    //         wgpu::BindGroupEntry {
    //             binding: 0,
    //             resource: uniform_buf.as_entire_binding(),
    //         },
    //         wgpu::BindGroupEntry {
    //             binding: 1,
    //             resource: wgpu::BindingResource::TextureView(texture_view),
    //         },
    //         wgpu::BindGroupEntry {
    //             binding: 2,
    //             resource: wgpu::BindingResource::Sampler(sampler),
    //         },
    //     ],
    //     label: None,
    // });
    //
    // let entity_uniform_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
    //     layout: &entity_uniform_bg_layout,
    //     entries: &[
    //         wgpu::BindGroupEntry {
    //             binding: 0,
    //             resource: uniform_buf.as_entire_binding(),
    //         },
    //         wgpu::BindGroupEntry {
    //             binding: 1,
    //             resource: wgpu::BindingResource::TextureView(texture_view),
    //         },
    //         wgpu::BindGroupEntry {
    //             binding: 2,
    //             resource: wgpu::BindingResource::Sampler(sampler),
    //         },
    //     ],
    //     label: None,
    // });
    //
    // let entity_texture_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
    //     layout: &entity_texture_bg_layout,
    //     entries: &[
    //         wgpu::BindGroupEntry {
    //             binding: 0,
    //             resource: uniform_buf.as_entire_binding(),
    //         },
    //         wgpu::BindGroupEntry {
    //             binding: 1,
    //             resource: wgpu::BindingResource::TextureView(texture_view),
    //         },
    //         wgpu::BindGroupEntry {
    //             binding: 2,
    //             resource: wgpu::BindingResource::Sampler(sampler),
    //         },
    //     ],
    //     label: None,
    // });

    let light_bg_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
        label: None,
    });

    let light_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &light_bg_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: light_buffer.as_entire_binding(),
        }],
        label: None,
    });

    let camera_bg_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
        label: Some("camera_bind_group_layout"),
    });

    let camera_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &camera_bg_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: camera_buffer.as_entire_binding(),
        }],
        label: Some("camera_bind_group"),
    });

    BindGroupData {
        texture_bg_layout,
        // diffuse_bg_layout
        // entity_bg_layout,
        camera_bg_layout,
        // entity_uniform_bg_layout,
        // entity_texture_bg_layout,
        // texture_bg,
        // diffuse_bg_layout: BindGroupLayout,
        // entity_bg,
        // entity_uniform_bg,
        // entity_texture_bg,
        light_bg_layout,
        light_bg,
        camera_bg,
    }
}

fn add_scene_entities(entities: &mut Vec<Entity>, meshes: &mut Vec<Mesh>) {
    let cuboid1 = Brush::make_cuboid(10., 10., 10.);
    let mesh1 = Mesh::from_brush(cuboid1);

    let entity1 = Entity {
        mesh: MESH_I.fetch_add(1, Ordering::Release),
        position: Vec3::new(70., 5., 20.),
        rotation: Quaternion::new_identity(),
        scale: 1.,
    };

    entities.push(entity1);
    meshes.push(mesh1);

    let floor_brush = Brush::make_cuboid(100., -1., 100.);
    let floor_mesh = Mesh::from_brush(floor_brush);

    let floor_entity = Entity {
        mesh: MESH_I.fetch_add(1, Ordering::Release),
        position: Vec3::new(0., -0.5, 0.),
        rotation: Quaternion::new_identity(),
        scale: 1.,
    };

    // entities.push(floor_entity);
    // meshes.push(floor_mesh);
}

pub trait DrawModel<'a> {
    fn draw_mesh(
        &mut self,
        mesh: &'a MeshWgpu,
        material: &'a Material,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );
    fn draw_mesh_instanced(
        &mut self,
        mesh: &'a MeshWgpu,
        material: &'a Material,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );

    fn draw_model(
        &mut self,
        model: &'a Model,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );
    fn draw_model_instanced(
        &mut self,
        model: &'a Model,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );
    fn draw_model_instanced_with_material(
        &mut self,
        model: &'a Model,
        material: &'a Material,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawModel<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_mesh(
        &mut self,
        mesh: &'b MeshWgpu,
        material: &'b Material,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'b wgpu::BindGroup,
    ) {
        self.draw_mesh_instanced(mesh, material, 0..1, camera_bind_group, light_bind_group);
    }

    fn draw_mesh_instanced(
        &mut self,
        mesh: &'b MeshWgpu,
        material: &'b Material,
        instances: Range<u32>,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'b wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(0, mesh.vertex_buf.slice(..));
        self.set_index_buffer(mesh.index_buf.slice(..), wgpu::IndexFormat::Uint32);
        self.set_bind_group(0, &material.bind_group, &[]);
        self.set_bind_group(1, camera_bind_group, &[]);
        self.set_bind_group(2, light_bind_group, &[]);
        self.draw_indexed(0..mesh.vertex_count, 0, instances);
    }

    fn draw_model(
        &mut self,
        model: &'b Model,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'b wgpu::BindGroup,
    ) {
        self.draw_model_instanced(model, 0..1, camera_bind_group, light_bind_group);
    }

    fn draw_model_instanced(
        &mut self,
        model: &'b Model,
        instances: Range<u32>,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'b wgpu::BindGroup,
    ) {
        for mesh in &model.meshes {
            // todo: Put back
            // let material = &model.materials[mesh.material];
            // self.draw_mesh_instanced(
            //     mesh,
            //     material,
            //     instances.clone(),
            //     camera_bind_group,
            //     light_bind_group,
            // );
        }
    }

    fn draw_model_instanced_with_material(
        &mut self,
        model: &'b Model,
        material: &'b Material,
        instances: Range<u32>,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'b wgpu::BindGroup,
    ) {
        for mesh in &model.meshes {
            // todo: Put back!
            // self.draw_mesh_instanced(
            //     mesh,
            //     material,
            //     instances.clone(),
            //     camera_bind_group,
            //     light_bind_group,
            // );
        }
    }
}

pub trait DrawLight<'a> {
    fn draw_light_mesh(
        &mut self,
        mesh: &'a MeshWgpu,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );
    fn draw_light_mesh_instanced(
        &mut self,
        mesh: &'a MeshWgpu,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );

    fn draw_light_model(
        &mut self,
        model: &'a types_wgpu::Model,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );
    fn draw_light_model_instanced(
        &mut self,
        model: &'a types_wgpu::Model,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawLight<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_light_mesh(
        &mut self,
        mesh: &'b MeshWgpu,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'b wgpu::BindGroup,
    ) {
        self.draw_light_mesh_instanced(mesh, 0..1, camera_bind_group, light_bind_group);
    }

    fn draw_light_mesh_instanced(
        &mut self,
        mesh: &'b MeshWgpu,
        instances: Range<u32>,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'b wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(0, mesh.vertex_buf.slice(..));
        self.set_index_buffer(mesh.index_buf.slice(..), wgpu::IndexFormat::Uint32);
        self.set_bind_group(0, camera_bind_group, &[]);
        self.set_bind_group(1, light_bind_group, &[]);
        self.draw_indexed(0..mesh.vertex_count, 0, instances);
    }

    fn draw_light_model(
        &mut self,
        model: &'b types_wgpu::Model,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'b wgpu::BindGroup,
    ) {
        self.draw_light_model_instanced(model, 0..1, camera_bind_group, light_bind_group);
    }
    fn draw_light_model_instanced(
        &mut self,
        model: &'b types_wgpu::Model,
        instances: Range<u32>,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'b wgpu::BindGroup,
    ) {
        for mesh in &model.meshes {
            self.draw_light_mesh_instanced(
                mesh,
                instances.clone(),
                camera_bind_group,
                light_bind_group,
            );
        }
    }
}
