use crate::{
    lin_alg::{Vec3},
    types::{VEC3_SIZE, F32_SIZE},
};

pub const LIGHTING_SIZE: usize = 3 * VEC3_SIZE + 2 * F32_SIZE;

#[derive(Debug, Clone)]
pub struct Lighting {
    ambient_color: Vec3,
    ambient_brightness: f32,
    diffuse_color: Vec3,
    diffuse_brightness: f32,
    diffuse_dir: Vec3,
    // todo: For now, just global lighting.
}

impl Default for Lighting {
    fn default() -> Self {
        Self {
            ambient_color: Vec3::new(1., 0., 1.),
            ambient_brightness: 0.1,
            diffuse_color: Vec3::new(1., 1., 1.),
            diffuse_brightness: 0.5,
            diffuse_dir: Vec3::new(0., 1., 0.),
        }
    }
}

impl Lighting {
    pub fn to_bytes(&self) -> [u8; LIGHTING_SIZE] {
        let mut result = [0; LIGHTING_SIZE];

        // 12 is vec3 size in bytes.
        result[0..12].clone_from_slice(&self.ambient_color.to_bytes());
        result[12..16].clone_from_slice(&self.ambient_brightness.to_le_bytes());
        result[16..28].clone_from_slice(&self.diffuse_color.to_bytes());
        result[28..32].clone_from_slice(&self.diffuse_brightness.to_le_bytes());
        result[32..LIGHTING_SIZE].clone_from_slice(&self.diffuse_dir.to_bytes());

        result
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