//! Code to manage the camera.

use core::f32::consts::TAU;

use crate::{
    init_graphics::{FWD_VEC, RIGHT_VEC, UP_VEC},
    lin_alg::{Mat4, Quaternion, Vec3},
    types::{MAT4_SIZE, VEC3_UNIFORM_SIZE},
};

// cam size is only the parts we pass to the shader.
// For each of the 4 matrices in the camera, plus a padded vec3 for position.
pub const CAM_UNIFORM_SIZE: usize = MAT4_SIZE + VEC3_UNIFORM_SIZE;

/// This is the component of the camrea that
pub struct CameraUniform {
    /// The projection matrix only changes when camera properties (fov, aspect, near, far)
    /// change, store it.
    /// By contrast, the view matrix changes whenever we changed position or orientation.
    pub proj_view_mat: Mat4,
    pub position: Vec3,
}

impl CameraUniform {
    pub fn to_bytes(&self) -> [u8; CAM_UNIFORM_SIZE] {
        let mut result = [0; CAM_UNIFORM_SIZE];

        // 64 is mat4 size in bytes.
        result[0..MAT4_SIZE].clone_from_slice(&self.proj_view_mat.to_bytes());
        result[MAT4_SIZE..CAM_UNIFORM_SIZE].clone_from_slice(&self.position.to_bytes_uniform());

        result
    }
}

#[derive(Clone, Debug)]
pub struct Camera {
    pub fov_y: f32,  // Vertical field of view in radians.
    pub aspect: f32, // width / height.
    pub near: f32,
    pub far: f32,
    /// Position shifts all points prior to the camera transform; this is what
    /// we adjust with move keys.
    pub position: Vec3,
    pub orientation: Quaternion,
    /// We store the projection matrix here since it only changes when we change the camera cfg.
    pub proj_mat: Mat4,
}

impl Camera {
    /// Update the stored projection matrices. Run this whenever we change camera parameters like
    /// FOV and aspect ratio.
    pub fn to_uniform(&self) -> CameraUniform {
        // todo: How does the inverted proj mat work?
        CameraUniform {
            position: self.position,
            // todo: Generate view mat seprately, only when cam changes?
            proj_view_mat: self.proj_mat.clone() * self.view_mat(),
        }
    }

    pub fn update_proj_mat(&mut self) {
        self.proj_mat = Mat4::new_perspective_rh(self.fov_y, self.aspect, self.near, self.far);
    }

    /// Calculate the view matrix: This is a translation of the negative coordinates of the camera's
    /// position, applied before the camera's rotation.
    pub fn view_mat(&self) -> Mat4 {
        self.orientation.inverse().to_matrix() * Mat4::new_translation(self.position * -1.)
    }

    pub fn view_size(&self, far: bool) -> (f32, f32) {
        // Calculate the projected window width and height, using basic trig.
        let dist = if far { self.far } else { self.near };

        let width = 2. * dist * (self.fov_y * self.aspect / 2.).tan();
        let height = 2. * dist * (self.fov_y / 2.).tan();
        (width, height)
    }
}

impl Default for Camera {
    fn default() -> Self {
        let mut result = Self {
            position: Vec3::new(0., 0., -5.),
            orientation: Quaternion::new_identity(),
            fov_y: TAU / 3., // Vertical field of view in radians.
            aspect: 4. / 3., // width / height.
            near: 1.,
            far: 100.,
            proj_mat: Mat4::new_identity(),
        };

        result.update_proj_mat();
        result
    }
}
