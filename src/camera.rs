//! Code to manage the camera.

use core::f32::consts::TAU;

use crate::{
    init_graphics::{FWD_VEC, RIGHT_VEC, UP_VEC},
    lin_alg::{Mat4, Quaternion, Vec3},
    types::{MAT4_SIZE, VEC3_UNIFORM_SIZE},
};

// cam size is only the parts we pass to the shader.
// For each of the 4 matrices in the camera, plus a padded vec3 for position.
pub const CAM_UNIFORM_SIZE: usize = 2 * MAT4_SIZE + VEC3_UNIFORM_SIZE;

/// This is the component of the camrea that
pub struct CameraUniform {
    /// The projection matrix only changes when camera properties (fov, aspect, near, far)
    /// change, store it.
    /// By contrast, the view matrix changes whenever we changed position or orientation.
    pub proj_view_mat: Mat4,
    /// We us the inverse project matrix for... lighting?
    pub proj_mat_inv: Mat4,
    pub position: Vec3,
    // pub view_mat: Mat4,
}

impl CameraUniform {
    pub fn to_bytes(&self) -> [u8; CAM_UNIFORM_SIZE] {
        let mut result = [0; CAM_UNIFORM_SIZE];

        // 64 is mat4 size in bytes.
        result[0..64].clone_from_slice(&self.proj_view_mat.to_bytes());
        result[64..128].clone_from_slice(&self.proj_mat_inv.to_bytes());
        result[128..132].clone_from_slice(&self.position.x.to_ne_bytes());
        result[132..136].clone_from_slice(&self.position.y.to_ne_bytes());
        result[136..140].clone_from_slice(&self.position.z.to_ne_bytes());
        result[140..144].clone_from_slice(&[0_u8; 4]); // Vec3 pad

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
        // self.projection_mat_inv = self.projection_mat.inverse().unwrap();
        let proj_mat_inv = Mat4::new_identity(); // todo temp

        // todo: How does the inverted proj mat work?
        CameraUniform {
            position: self.position,
            proj_view_mat: self.proj_mat.clone() * self.view_mat(),
            proj_mat_inv,
        }
    }

    pub fn update_proj_mat(&mut self) {
        self.proj_mat = Mat4::new_perspective_rh(self.fov_y, self.aspect, self.near, self.far);
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
        Self {
            position: Vec3::new(0., 2., 10.),
            orientation: Quaternion::new_identity(),
            fov_y: TAU / 3., // Vertical field of view in radians.
            aspect: 4. / 3., // width / height.
            near: 1.,
            far: 100.,
            proj_mat: Mat4::new_identity(),
        }
    }
}
