//! Handles keyboard and mouse input, eg for moving the camera.

use crate::{
    camera::Camera,
    init_graphics::{FWD_VEC, RIGHT_VEC, UP_VEC},
    types::InputSettings,
};

use lin_alg2::f32::{Quaternion, Vec3};

// todo: remove Winit from this module if you can, and make it agnostic?
use winit::event::{DeviceEvent, ElementState};

const MOUSE_1_ID: u32 = 1;

#[derive(Default, Debug)]
pub struct InputsCommanded {
    pub fwd: bool,
    pub back: bool,
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    pub roll_ccw: bool,
    pub roll_cw: bool,
    pub mouse_delta_x: f32,
    pub mouse_delta_y: f32,
    pub run: bool,
    pub free_look: bool,
}


impl InputsCommanded {
    /// Return true if there are any inputs.
    pub fn inputs_present(&self) -> bool {
        const EPS: f32 = 0.00001;
        // Note; We don't include `run` or `free_look` here, since it's a modifier.
        self.fwd
            || self.back
            || self.left
            || self.right
            || self.up
            || self.down
            || self.roll_ccw
            || self.roll_cw
            || self.mouse_delta_x.abs() > EPS
            || self.mouse_delta_y.abs() > EPS
    }
}

/// Modifies the commanded inputs in place; triggered by a single input event.
/// dt is in seconds.
/// pub(crate) fn handle_event(event: DeviceEvent, cam: &mut Camera, input_settings: &InputSettings, dt: f32) {
pub(crate) fn add_input_cmd(event: DeviceEvent, inputs: &mut InputsCommanded) {
    match event {
        DeviceEvent::Key(key) => {
            if key.state == ElementState::Pressed {
                match key.scancode {
                    17 => {
                        // W
                        inputs.fwd = true;
                    }
                    31 => {
                        // S
                        inputs.back = true;
                    }
                    32 => {
                        // D
                        inputs.right = true;
                    }
                    30 => {
                        // A
                        inputs.left = true;
                    }
                    57 => {
                        // Space
                        inputs.up = true;
                    }
                    46 => {
                        // C
                        inputs.down = true;
                    }
                    16 => {
                        // Q
                        inputs.roll_ccw = true;
                    }
                    18 => {
                        // E
                        inputs.roll_cw = true;
                    }
                    42 => {
                        // Shift
                        inputs.run = true;
                    }
                    _ => (),
                }
            } else if key.state == ElementState::Released {
                // todo: DRY
                match key.scancode {
                    17 => {
                        inputs.fwd = false;
                    }
                    31 => {
                        inputs.back = false;
                    }
                    32 => {
                        inputs.right = false;
                    }
                    30 => {
                        inputs.left = false;
                    }
                    57 => {
                        inputs.up = false;
                    }
                    46 => {
                        inputs.down = false;
                    }
                    16 => {
                        inputs.roll_ccw = false;
                    }
                    18 => {
                        inputs.roll_cw = false;
                    }
                    42 => {
                        inputs.run = false;
                    }
                    _ => (),
                }
            }
        }
        DeviceEvent::Button { button, state } => match button {
            MOUSE_1_ID => match state {
                ElementState::Pressed => inputs.free_look = true,
                ElementState::Released => inputs.free_look = false,
            },
            _ => (),
        },
        DeviceEvent::MouseMotion { delta } => {
            inputs.mouse_delta_x += delta.0 as f32;
            inputs.mouse_delta_y += delta.1 as f32;
        }
        _ => (),
    }
}

/// Adjust the camera orientation and position.
/// todo: copyied from `peptide`'s Bevy interface.
pub fn adjust_camera(
    cam: &mut Camera,
    inputs: &InputsCommanded,
    input_settings: &InputSettings,
    dt: f32,
) {
    let mut move_amt: f32 = input_settings.move_sens * dt;
    let rotate_amt: f32 = input_settings.rotate_sens * dt;
    let mut rotate_key_amt: f32 = input_settings.rotate_key_sens * dt;

    // todo: This split is where you can decouple WGPU-specific code from general code.

    let mut cam_moved = false;
    let mut cam_rotated = false;

    let mut movement_vec = Vec3::new_zero();

    if inputs.run {
        move_amt *= input_settings.run_factor;
        rotate_key_amt *= input_settings.run_factor;
    }

    if inputs.fwd {
        movement_vec.z += move_amt;
        cam_moved = true;
    } else if inputs.back {
        movement_vec.z -= move_amt;
        cam_moved = true;
    }

    if inputs.right {
        movement_vec.x += move_amt;
        cam_moved = true;
    } else if inputs.left {
        movement_vec.x -= move_amt;
        cam_moved = true;
    }

    if inputs.up {
        movement_vec.y += move_amt;
        cam_moved = true;
    } else if inputs.down {
        movement_vec.y -= move_amt;
        cam_moved = true;
    }

    let fwd = cam.orientation.rotate_vec(FWD_VEC);
    // todo: Why do we need to reverse these?
    let up = cam.orientation.rotate_vec(UP_VEC * -1.);
    let right = cam.orientation.rotate_vec(RIGHT_VEC * -1.);

    let mut rotation = Quaternion::new_identity();

    // todo: Why do we need to reverse these?
    if inputs.roll_cw {
        rotation = Quaternion::from_axis_angle(fwd, -rotate_key_amt);
        cam_rotated = true;
    } else if inputs.roll_ccw {
        rotation = Quaternion::from_axis_angle(fwd, rotate_key_amt);
        cam_rotated = true;
    }

    let eps = 0.00001;

    if inputs.free_look {
        if inputs.mouse_delta_x.abs() > eps || inputs.mouse_delta_y.abs() > eps {
            // todo: Why do we have the negative signs here?
            rotation = Quaternion::from_axis_angle(up, -inputs.mouse_delta_x * rotate_amt)
                * Quaternion::from_axis_angle(right, -inputs.mouse_delta_y * rotate_amt)
                * rotation;

            cam_rotated = true;
        }
    }

    if cam_moved {
        cam.position = cam.position + cam.orientation.rotate_vec(movement_vec);
    }

    if cam_rotated {
        cam.orientation = rotation * cam.orientation;
    }
}
