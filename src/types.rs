//! https://sotrh.github.io/learn-wgpu/beginner/tutorial9-models/#rendering-a-mesh

use std::ops::Range;

use crate::{
    lighting::PointLight,
    lin_alg::{Mat4, Quaternion, Vec3},
};

// These sizes are in bytes. We do this, since that's the data format expected by the shader.
pub const F32_SIZE: usize = 4;

pub const VEC3_SIZE: usize = 3 * F32_SIZE;
pub const VEC3_UNIFORM_SIZE: usize = 4 * F32_SIZE;
pub const VERTEX_SIZE: usize = 14 * F32_SIZE;
pub const MAT4_SIZE: usize = 16 * F32_SIZE;
pub const MAT3_SIZE: usize = 9 * F32_SIZE;

pub const INSTANCE_SIZE: usize = MAT4_SIZE + MAT3_SIZE;

#[derive(Clone, Copy, Debug)]
/// Example attributes: https://github.com/bevyengine/bevy/blob/main/crates/bevy_render/src/mesh/mesh/mod.rs#L56
/// // todo: Vec3 vs arrays?
pub struct ModelVertex {
    /// Where the vertex is located in space
    pub position: [f32; 3],
    /// AKA UV mapping. https://en.wikipedia.org/wiki/UV_mapping
    pub tex_coords: [f32; 2],
    /// The direction the vertex normal is facing in
    pub normal: Vec3,
    /// "Tangent and Binormal vectors are vectors that are perpendicular to each other
    /// and the normal vector which essentially describe the direction of the u,v texture
    /// coordinates with respect to the surface that you are trying to render. Typically
    /// they can be used alongside normal maps which allow you to create sub surface
    /// lighting detail to your model(bumpiness)."
    pub tangent: [f32; 3],
    pub bitangent: [f32; 3],
}

impl ModelVertex {
    /// Initialize position; change the others after init.
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            position: [x, y, z],
            tex_coords: [0., 0.],
            normal: Vec3::new_zero(),
            tangent: [0., 0., 0.],
            bitangent: [0., 0., 0.],
        }
    }

    pub fn to_bytes(&self) -> [u8; VERTEX_SIZE] {
        let mut result = [0; VERTEX_SIZE];

        result[0..4].clone_from_slice(&self.position[0].to_ne_bytes());
        result[4..8].clone_from_slice(&self.position[1].to_ne_bytes());
        result[8..12].clone_from_slice(&self.position[2].to_ne_bytes());
        result[12..16].clone_from_slice(&self.tex_coords[0].to_ne_bytes());
        result[16..20].clone_from_slice(&self.tex_coords[1].to_ne_bytes());

        result[20..32].clone_from_slice(&self.normal.to_bytes_vertex());

        result[32..36].clone_from_slice(&self.tangent[0].to_ne_bytes());
        result[36..40].clone_from_slice(&self.tangent[1].to_ne_bytes());
        result[40..44].clone_from_slice(&self.tangent[2].to_ne_bytes());
        result[44..48].clone_from_slice(&self.bitangent[0].to_ne_bytes());
        result[48..52].clone_from_slice(&self.bitangent[1].to_ne_bytes());
        result[52..56].clone_from_slice(&self.bitangent[2].to_ne_bytes());

        result
    }

    // todo: This probably shouldn't be in this module, which is backend-agnostic-ish.
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: VERTEX_SIZE as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // position
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // tex_coords
                wgpu::VertexAttribute {
                    offset: VEC3_SIZE as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                // normal
                wgpu::VertexAttribute {
                    offset: (2 * F32_SIZE + VEC3_SIZE) as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // tangent
                wgpu::VertexAttribute {
                    offset: (2 * F32_SIZE + 2 * VEC3_SIZE) as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // bitangent
                wgpu::VertexAttribute {
                    offset: (2 * F32_SIZE + 3 * VEC3_SIZE) as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

/// Instances allow the GPU to render the same object multiple times.
/// "Instancing allows us to draw the same object multiple times with different properties
/// (position, orientation, size, color, etc.). "
pub struct Instance {
    pub position: Vec3,
    pub rotation: Quaternion,
    pub scale: f32,
}

impl Instance {
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: INSTANCE_SIZE as wgpu::BufferAddress,
            // We need to switch from using a step mode of Vertex to Instance
            // This means that our shaders will only change to use the next
            // instance when the shader starts processing a new instance
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // A mat4 takes up 4 vertex slots as it is technically 4 vec4s. We need to define a slot
                // for each vec4. We'll have to reassemble the mat4 in
                // the shader.
                wgpu::VertexAttribute {
                    offset: (F32_SIZE * 4) as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: (F32_SIZE * 8) as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: (F32_SIZE * 12) as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: (F32_SIZE * 16) as wgpu::BufferAddress,
                    shader_location: 9,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: (F32_SIZE * 19) as wgpu::BufferAddress,
                    shader_location: 10,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: (F32_SIZE * 22) as wgpu::BufferAddress,
                    shader_location: 11,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }

    /// Converts to a model matrix
    pub fn to_bytes(&self) -> [u8; INSTANCE_SIZE] {
        let mut result = [0; INSTANCE_SIZE];

        let model_mat = Mat4::new_translation(self.position)
            * self.rotation.to_matrix()
            * Mat4::new_scaler(self.scale);

        let normal_mat = self.rotation.to_matrix3();

        result[0..MAT4_SIZE].clone_from_slice(&model_mat.to_bytes());
        result[MAT4_SIZE..INSTANCE_SIZE].clone_from_slice(&normal_mat.to_bytes());

        result
    }
}

// todo: This shouldn't have WGP types in it.
pub struct Mesh {
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_elements: u32,
    pub material: usize,
}

// impl<'a> Mesh {
//     // pub fn draw_instanced<'a, 'b>(
//     //     &'b self,
//     //     rpass: &mut wgpu::RenderPass<'a>,
//     //     // material: &'b Material,
//     //     instances: Range<u32>,
//     //     camera_bind_group: &'b wgpu::BindGroup,
//     //     // light_bind_group: &'b wgpu::BindGroup,
//     // ) {
//     pub fn draw_instanced(
//         &'a self,
//         rpass: &mut wgpu::RenderPass<'a>,
//         // material: &'b Material,
//         instances: Range<u32>,
//         camera_bind_group: &'a wgpu::BindGroup,
//         // light_bind_group: &'b wgpu::BindGroup,
//     ) {
//         rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
//         rpass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
//         // rpass.set_bind_group(0, &material.bind_group, &[]);
//         rpass.set_bind_group(0, camera_bind_group, &[]);
//         // rpass.set_bind_group(2, light_bind_group, &[]);
//         rpass.draw_indexed(0..self.num_elements, 0, instances);
//     }
// }

/// Represents an entity in the world. This is not fundamental to the WGPU system.
#[derive(Clone, Debug)]
pub struct Entity {
    /// Index of the mesh this entity references. (or perhaps its index?)
    pub mesh: usize,
    /// Position in the world, relative to world origin
    pub position: Vec3,
    /// Rotation, relative to up.
    pub orientation: Quaternion,
    pub scale: f32, // 1.0 is original.
}

#[derive(Clone, Debug)]
/// A brush is a geometry representation that can be converted to a mesh. Unlike a mesh, it's not
/// designed to be passed directly to the GPU.
pub struct Brush {
    pub vertices: Vec<ModelVertex>,
    /// Faces are defined in terms of vertex index, and must be defined in an order of adjacent
    /// edges. (LH or RH?)
    pub faces: Vec<Vec<usize>>,
}

impl Brush {
    pub fn new(vertices: Vec<ModelVertex>, faces: Vec<Vec<usize>>) -> Self {
        Self { vertices, faces }
    }

    pub fn make_cuboid(x: f32, y: f32, z: f32) -> Self {
        // todo: Normals and/or tex coords?

        // Divide by 2 to get coordinates from len, width, heigh
        let x = x / 2.;
        let y = y / 2.;
        let z = z / 2.;

        Self {
            vertices: vec![
                // top
                ModelVertex::new(x, y, z),
                ModelVertex::new(x, y, -z),
                ModelVertex::new(-x, y, -z),
                ModelVertex::new(-x, y, z),
                // bottom
                ModelVertex::new(x, -y, z),
                ModelVertex::new(x, -y, -z),
                ModelVertex::new(-x, -y, -z),
                ModelVertex::new(-x, -y, z),
            ],

            faces: vec![
                // top
                vec![0, 1, 2, 3],
                // bottom
                vec![4, 5, 6, 7],
                // left
                vec![2, 3, 6, 7],
                // right
                vec![0, 1, 4, 5],
                // front
                vec![0, 3, 4, 7],
                // back
                vec![1, 2, 5, 6],
            ],
        }
    }

    // pub fn make_cuboid(x: f32, y: f32, z: f32) -> Self {

    // }

    // pub fn compute_normals(&mut self) {
    //     for face in &self.faces_vert {
    //         // todo make sure these aren't reversed!
    //         let line1 = self.vertices[&face[1]].subtract(&self.vertices[&face[0]]);
    //         let line2 = self.vertices[&face[2]].subtract(&self.vertices[&face[0]]);
    //         normals.push(line1.cross(&line2));
    //     }
    //
    //     self.normals = normals;
    // }
}

#[derive(Clone, Debug, Default)]
pub struct Scene {
    pub entities: Vec<Entity>,
    pub lights: Vec<PointLight>,
}
