use crate::types::{F32_SIZE, VEC3_UNIFORM_SIZE};

use lin_alg2::f32::Vec3;

pub const LIGHTING_SIZE: usize = 3 * VEC3_UNIFORM_SIZE + 2 * F32_SIZE;
pub const POINT_LIGHT_SIZE: usize = 2 * VEC3_UNIFORM_SIZE + F32_SIZE;

#[derive(Debug, Clone)]
pub struct Lighting {
    ambient_color: Vec3,
    ambient_intensity: f32,
    diffuse_color: Vec3,
    diffuse_intensity: f32,
    diffuse_dir: Vec3,
    point_lights: Vec<PointLight>,
}

impl Default for Lighting {
    fn default() -> Self {
        Self {
            ambient_color: Vec3::new(1., 1., 1.).to_normalized(),
            ambient_intensity: 0.05,
            diffuse_color: Vec3::new(1., 1., 1.).to_normalized(),
            diffuse_intensity: 1.,
            diffuse_dir: Vec3::new(0., -1., 0.).to_normalized(),
            point_lights: Vec::new(),
        }
    }
}

impl Lighting {
    pub fn to_bytes(&self) -> [u8; LIGHTING_SIZE] {
        let mut result = [0; LIGHTING_SIZE];

        // 16 is vec3 size in bytes, including padding.
        result[0..16].clone_from_slice(&self.ambient_color.to_bytes_uniform());
        result[16..20].clone_from_slice(&self.ambient_intensity.to_ne_bytes());
        result[20..36].clone_from_slice(&self.diffuse_color.to_bytes_uniform());
        result[36..40].clone_from_slice(&self.diffuse_intensity.to_ne_bytes());
        result[40..LIGHTING_SIZE].clone_from_slice(&self.diffuse_dir.to_bytes_uniform());

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
pub struct PointLight {
    // A point light source
    pub type_: LightType,
    pub position: Vec3,
    pub color: [f32; 4],
    pub intensity: f32,
    // todo: FOV? range?
    // shadow_map
}

impl PointLight {
    /// todo: assumes point source for now; ignore type_ field.
    pub fn to_bytes(&self) -> [u8; POINT_LIGHT_SIZE] {
        let mut result = [0; POINT_LIGHT_SIZE];

        // 16 is vec3 size in bytes, including padding.
        result[0..16].clone_from_slice(&self.position.to_bytes_uniform());

        result[16..20].clone_from_slice(&self.color[0].to_ne_bytes());
        result[20..24].clone_from_slice(&self.color[1].to_ne_bytes());
        result[24..28].clone_from_slice(&self.color[2].to_ne_bytes());
        result[28..32].clone_from_slice(&self.color[3].to_ne_bytes());

        result[32..POINT_LIGHT_SIZE].clone_from_slice(&self.intensity.to_ne_bytes());

        result
    }
}
