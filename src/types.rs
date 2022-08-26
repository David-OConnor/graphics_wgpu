//! https://sotrh.github.io/learn-wgpu/beginner/tutorial9-models/#rendering-a-mesh

use std::f32::consts::TAU;

use crate::{
    init_graphics::{FWD_VEC, RIGHT_VEC, UP_VEC},
    lin_alg::{Mat4, Quaternion, Vec3},
};

// These sizes are in bytes. We do this, since that's the data format expected by the shader.
pub const VERTEX_SIZE: usize = 14 * 4;

pub const MAT4_SIZE: usize = 16 * 4;
// cam size is only the parts we pass to the shader.
// For each of the 4 matrices in the camera, plus a padded vec3 for position.
pub const CAM_SIZE: usize = 3 * MAT4_SIZE + 4 * 4;

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
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    // todo: Should this be of the prev (3), or this? (2)
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
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

#[derive(Clone, Debug)]
pub struct Camera {
    // todo: Consider a substruct either of the uniform data fields, or the non-uniform
    // todo data fields.
    pub fov_y: f32,  // Vertical field of view in radians.
    pub aspect: f32, // width / height.
    pub near: f32,
    pub far: f32,
    /// Position shifts all points prior to the camera transform; this is what
    /// we adjust with move keys.
    pub position: Vec3,
    pub orientation: Quaternion,
    /// The projection matrix only changes when camera properties (fov, aspect, near, far)
    /// change, store it.
    /// By contrast, the view matrix changes whenever we changed position or orientation.
    pub projection_mat: Mat4,
    /// We us the inverse project matrix for... lighting?
    pub projection_mat_inv: Mat4,
}

impl Camera {
    /// Update the stored projection matrices. Run this whenever we change camera parameters like
    /// FOV and aspect ratio.
    pub fn update_proj_mats(&mut self) {
        // todo: CLean this up once you sort out your proj mat logic!!
        self.projection_mat =
            Mat4::new_perspective_rh(self.fov_y, self.aspect, self.near, self.far);

        // self.projection_mat_inv = self.projection_mat.inverse().unwrap();

        // todo: How does the inverted proj mat work?
    }

    /// Calculate the view matrix.
    #[rustfmt::skip]
    pub fn view_mat(&self) -> Mat4 {
        let mat = self.orientation.to_matrix();

        Mat4::new([
            mat.data[0], mat.data[3], mat.data[6], 0.,
            mat.data[1], mat.data[4], mat.data[7], 0.,
            mat.data[2], mat.data[5], mat.data[8], 0.,
            -self.position.dot(RIGHT_VEC), -self.position.dot(UP_VEC), self.position.dot(FWD_VEC), 1.,
        ])
    }

    pub fn to_bytes(&self) -> [u8; CAM_SIZE] {
        let view = self.view_mat();

        let mut result = [0; CAM_SIZE];

        result[0..MAT4_SIZE].clone_from_slice(&self.projection_mat.to_bytes());
        result[MAT4_SIZE..2 * MAT4_SIZE].clone_from_slice(&self.projection_mat_inv.to_bytes());
        result[2 * MAT4_SIZE..3 * MAT4_SIZE].clone_from_slice(&self.view_mat().to_bytes());

        result[3 * MAT4_SIZE..CAM_SIZE - 12].clone_from_slice(&self.position.y.to_le_bytes());
        result[3 * MAT4_SIZE - 12..CAM_SIZE - 8].clone_from_slice(&self.position.y.to_le_bytes());
        result[3 * CAM_SIZE - 8..CAM_SIZE - 4].clone_from_slice(&self.position.z.to_le_bytes());
        result[3 * CAM_SIZE - 4..CAM_SIZE].clone_from_slice(&1.0_f32.to_le_bytes());

        result
    }

    pub fn view_size(&self, far: bool) -> (f32, f32) {
        // Calculate the projected window width and height, using basic trig.
        let dist = if far { self.far } else { self.near };

        let width = 2. * dist * (self.fov_y * self.aspect / 2.).tan();
        let height = 2. * dist * (self.fov_y / 2.).tan();
        (width, height)
    }

    // /// We only convert the parts we need in the shader.
    // pub fn to_bytes(&self) -> [u8; VERTEX_SIZE] {
    //     let mut result = [0; VERTEX_SIZE];
    //
    //     result[0..4].clone_from_slice(&self.position[0].to_le_bytes());
    //     result[4..8].clone_from_slice(&self.position[1].to_le_bytes());
    //     result[8..12].clone_from_slice(&self.position[2].to_le_bytes());
    //     result[12..16].clone_from_slice(&self.tex_coords[0].to_le_bytes());
    //     result[16..20].clone_from_slice(&self.tex_coords[1].to_le_bytes());
    //     result[20..24].clone_from_slice(&self.normal[0].to_le_bytes());
    //     result[24..28].clone_from_slice(&self.normal[1].to_le_bytes());
    //     result[28..32].clone_from_slice(&self.normal[2].to_le_bytes());
    //     result[32..36].clone_from_slice(&self.tangent[0].to_le_bytes());
    //     result[36..40].clone_from_slice(&self.tangent[1].to_le_bytes());
    //     result[40..44].clone_from_slice(&self.tangent[2].to_le_bytes());
    //     result[44..48].clone_from_slice(&self.bitangent[0].to_le_bytes());
    //     result[48..52].clone_from_slice(&self.bitangent[1].to_le_bytes());
    //     result[52..56].clone_from_slice(&self.bitangent[2].to_le_bytes());
    //
    //     result
    // }
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: Vec3::new(0., 2., 10.),
            orientation: Quaternion::new_identity(),
            fov_y: TAU / 3., // Vertical field of view in radians.
            aspect: 4. / 3., // width / height.
            near: 1.,
            far: 100.,
            projection_mat: Mat4::new_identity(),
            projection_mat_inv: Mat4::new_identity(),
        }
    }
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
