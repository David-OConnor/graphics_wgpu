use crate::types::{F32_SIZE, VEC3_UNIFORM_SIZE};

use lin_alg2::f32::Vec3;

// The extra 4 is due to uniform buffers needing ton be a multiple of 16 in size.
pub const LIGHTING_SIZE: usize = 4 * VEC3_UNIFORM_SIZE + 3 * F32_SIZE + 4;
// The extra 8 here for the same reason.
pub const POINT_LIGHT_SIZE: usize = 2 * VEC3_UNIFORM_SIZE + 8;

#[derive(Debug, Clone)]
/// We organize the fields in this order, and serialize them accordingly, to keep the buffer
/// from being too long while adhering to alignment rules.
pub struct Lighting {
    ambient_color: Vec3,
    diffuse_dir: Vec3,
    diffuse_color: Vec3,
    specular_color: Vec3,
    ambient_intensity: f32,
    diffuse_intensity: f32,
    specular_intensity: f32,
    pub point_lights: Vec<PointLight>,
}

impl Default for Lighting {
    fn default() -> Self {
        Self {
            ambient_color: Vec3::new(1., 1., 1.).to_normalized(),
            diffuse_dir: Vec3::new(0., -1., 0.).to_normalized(),
            diffuse_color: Vec3::new(1., 1., 1.).to_normalized(),
            specular_color: Vec3::new(1., 1., 1.).to_normalized(),
            ambient_intensity: 0.15,
            diffuse_intensity: 0.7,
            specular_intensity: 0.3,
            point_lights: Vec::new(),
        }
    }
}

impl Lighting {
    pub fn to_bytes(&self) -> [u8; LIGHTING_SIZE] {
        let mut result = [0; LIGHTING_SIZE];

        // 16 is vec3 size in bytes, including padding.
        result[0..VEC3_UNIFORM_SIZE].clone_from_slice(&self.ambient_color.to_bytes_uniform());
        result[VEC3_UNIFORM_SIZE..2 * VEC3_UNIFORM_SIZE]
            .clone_from_slice(&self.diffuse_dir.to_bytes_uniform());
        result[2 * VEC3_UNIFORM_SIZE..3 * VEC3_UNIFORM_SIZE]
            .clone_from_slice(&self.diffuse_color.to_bytes_uniform());
        result[3 * VEC3_UNIFORM_SIZE..4 * VEC3_UNIFORM_SIZE]
            .clone_from_slice(&self.specular_color.to_bytes_uniform());
        result[4 * VEC3_UNIFORM_SIZE..68].clone_from_slice(&self.ambient_intensity.to_ne_bytes());
        result[68..72].clone_from_slice(&self.diffuse_intensity.to_ne_bytes());
        result[72..76].clone_from_slice(&self.specular_intensity.to_ne_bytes());
        // Pad the whole buf to be a multiple of 16 in len.
        result[76..LIGHTING_SIZE].clone_from_slice(&[0_u8; 4]);

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
    // todo: FOV, and direction?
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

        result[32..POINT_LIGHT_SIZE - 8].clone_from_slice(&self.intensity.to_ne_bytes());
        result[POINT_LIGHT_SIZE - 8..POINT_LIGHT_SIZE].clone_from_slice(&[0; 8]);

        result
    }
}
