//! https://sotrh.github.io/learn-wgpu/beginner/tutorial9-models/#rendering-a-mesh

use crate::{
    lin_alg::{Quaternion, Vec3},
};

// These sizes are in bytes. We do this, since that's the data format expected by the shader.
pub const F32_SIZE: usize = 4;

pub const VERTEX_SIZE: usize = 14 * F32_SIZE;
pub const MAT4_SIZE: usize = 16 * F32_SIZE;
// cam size is only the parts we pass to the shader.
// For each of the 4 matrices in the camera, plus a padded vec3 for position.
pub const VEC3_SIZE: usize = 3 * F32_SIZE;

#[derive(Clone, Copy, Debug)]
/// Example attributes: https://github.com/bevyengine/bevy/blob/main/crates/bevy_render/src/mesh/mesh/mod.rs#L56
/// // todo: Vec3 vs arrays?
pub struct Vertex {
    /// Where the vertex is located in space
    pub position: [f32; 3],
    // pub position: Vec3,
    /// AKA UV mapping. https://en.wikipedia.org/wiki/UV_mapping
    pub tex_coords: [f32; 2],
    /// The direction the vertex normal is facing in
    pub normal: [f32; 3],
    /// "Tangent and Binormal vectors are vectors that are perpendicular to each other
    /// and the normal vector which essentially describe the direction of the u,v texture
    /// coordinates with respect to the surface that you are trying to render. Typically
    /// they can be used alongside normal maps which allow you to create sub surface
    /// lighting detail to your model(bumpiness)."
    pub tangent: [f32; 3],
    pub bitangent: [f32; 3],
}

impl Vertex {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            position: [x, y, z],
            tex_coords: [0., 0.],    // todo
            normal: [0., 0., 0.],    // todo
            tangent: [0., 0., 0.],   // todo
            bitangent: [0., 0., 0.], // todo
        }
    }

    pub fn to_bytes(&self) -> [u8; VERTEX_SIZE] {
        let mut result = [0; VERTEX_SIZE];

        result[0..4].clone_from_slice(&self.position[0].to_le_bytes());
        result[4..8].clone_from_slice(&self.position[1].to_le_bytes());
        result[8..12].clone_from_slice(&self.position[2].to_le_bytes());
        result[12..16].clone_from_slice(&self.tex_coords[0].to_le_bytes());
        result[16..20].clone_from_slice(&self.tex_coords[1].to_le_bytes());
        result[20..24].clone_from_slice(&self.normal[0].to_le_bytes());
        result[24..28].clone_from_slice(&self.normal[1].to_le_bytes());
        result[28..32].clone_from_slice(&self.normal[2].to_le_bytes());
        result[32..36].clone_from_slice(&self.tangent[0].to_le_bytes());
        result[36..40].clone_from_slice(&self.tangent[1].to_le_bytes());
        result[40..44].clone_from_slice(&self.tangent[2].to_le_bytes());
        result[44..48].clone_from_slice(&self.bitangent[0].to_le_bytes());
        result[48..52].clone_from_slice(&self.bitangent[1].to_le_bytes());
        result[52..56].clone_from_slice(&self.bitangent[2].to_le_bytes());

        result
    }

    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
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
                    offset: VEC3_SIZE as u64,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                // normal
                wgpu::VertexAttribute {
                    // offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    offset: (2 * F32_SIZE + VEC3_SIZE) as u64,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // tangent
                wgpu::VertexAttribute {
                    // offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    offset: (2 * F32_SIZE + 2 * VEC3_SIZE) as u64,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // bitangent
                wgpu::VertexAttribute {
                    // offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    offset: (2 * F32_SIZE + 3 * VEC3_SIZE) as u64,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

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

/// Mesh - represents geometry, and contains vertex and index buffers.
/// As a reference: https://github.com/bevyengine/bevy/blob/main/crates/bevy_render/src/mesh/mesh/mod.rs
#[derive(Debug)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    /// Each consecutive triplet of indices defines a triangle.
    pub indices: Vec<usize>,
}

// todo: You don't really want this bytemuck and WGPU buffer stuff here; use a vec etc.
impl Mesh {
    pub fn from_brush(brush: Brush) -> Self {
        // Create triangles from faces, which in turn reference vertex indices.
        // faces must be defined continously around their edge, with no jumps.
        let mut indices = Vec::new();

        // There may be more efficient algos for this, but this one is conceptually simple.
        for face in &brush.faces {
            if face.len() < 3 {
                panic!("Faces must have at least 3 vertices")
            }

            for i in 0..face.len() - 2 {
                // vertex 0 is used for all triangles.
                indices.push(face[0]);
                for j in 1..3 {
                    indices.push(face[i + j]);
                }
            }
        }

        Self {
            vertices: brush.vertices.clone(),
            indices,
        }
    }
}

#[derive(Clone, Debug)]
/// A brush is a geometry representation that can be converted to a mesh. Unlike a mesh, it's not
/// designed to be passed directly to the GPU.
pub struct Brush {
    pub vertices: Vec<Vertex>,
    /// Faces are defined in terms of vertex index, and must be defined in an order of adjacent
    /// edges. (LH or RH?)
    pub faces: Vec<Vec<usize>>,
}

impl Brush {
    pub fn new(vertices: Vec<Vertex>, faces: Vec<Vec<usize>>) -> Self {
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
                Vertex::new(x, y, z),
                Vertex::new(x, y, -z),
                Vertex::new(-x, y, -z),
                Vertex::new(-x, y, z),
                // bottom
                Vertex::new(x, -y, z),
                Vertex::new(x, -y, -z),
                Vertex::new(-x, -y, -z),
                Vertex::new(-x, -y, z),
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

#[derive(Debug, Clone)]
pub enum LightType {
    Omnidirectional,
    Directional(Vec3), // direction pointed at // todo: FOV?
    Diffuse,
}

#[derive(Clone, Debug)]
pub struct Light {
    // A point light source
    pub type_: LightType,
    pub position: Vec3,
    pub color: [f32; 4],
    pub intensity: f32,
    // todo: FOV? range?
    // shadow_map
}

#[derive(Clone, Debug, Default)]
pub struct Scene {
    pub entities: Vec<Entity>,
    pub lights: Vec<Light>,
}
