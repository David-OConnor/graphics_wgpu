//! Handles keyboard and mouse input, eg for moving the camera.

use crate::{
    camera::Camera,
    init_graphics::{DT, FWD_VEC, RIGHT_VEC, UP_VEC},
    lin_alg::{Quaternion, Vec3},
};

// todo: remove Winit from this module if you can, and make it agnostic?
use winit::event::{DeviceEvent, ElementState};

// These sensitivities are in units (position), or radians (orientation) per second.
const CAM_MOVE_SENS: f32 = 1.1;
const CAM_ROTATE_SENS: f32 = 0.3;
const CAM_ROTATE_KEY_SENS: f32 = 0.5;
// Move speed multiplier when the run modifier key is held.
const RUN_FACTOR: f32 = 3.;

#[derive(Default)]
struct InputsCommanded {
    fwd: bool,
    back: bool,
    left: bool,
    right: bool,
    up: bool,
    down: bool,
    roll_ccw: bool,
    roll_cw: bool,
    mouse_delta_x: f32,
    mouse_delta_y: f32,
    run: bool,
}

pub fn handle_event(event: DeviceEvent, cam: &mut Camera) {
    let mut inputs = InputsCommanded::default();

    match event {
        DeviceEvent::Key(key) => match key.scancode {
            17 => {
                // W
                println!("CAM: {:?}", cam);
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
        },

        DeviceEvent::MouseMotion { delta } => {
            inputs.mouse_delta_x = delta.0 as f32;
            inputs.mouse_delta_y = delta.1 as f32;
        }
        _ => (),
    }

    adjust_camera(cam, &inputs);
}

/// Adjust the camera orientation and position.
/// todo: copyied from `peptide`'s Bevy interface.
fn adjust_camera(cam: &mut Camera, inputs: &InputsCommanded) {
    let mut move_amt: f32 = CAM_MOVE_SENS * DT;
    const ROTATE_AMT: f32 = CAM_ROTATE_SENS * DT;
    let mut rotate_key_amt: f32 = CAM_ROTATE_KEY_SENS * DT;

    // todo: This split is where you can decouple WGPU-specific code from general code.

    let mut cam_moved = false;
    let mut cam_rotated = false;

    let mut movement_vec = Vec3::new_zero();

    if inputs.run {
        move_amt *= RUN_FACTOR;
        rotate_key_amt *= RUN_FACTOR;
    }

    if inputs.fwd {
        movement_vec.z -= move_amt; // todo: Backwards; why?
        cam_moved = true;
    } else if inputs.back {
        movement_vec.z += move_amt;
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
    if inputs.mouse_delta_x.abs() > eps || inputs.mouse_delta_y.abs() > eps {
        rotation = Quaternion::from_axis_angle(up, inputs.mouse_delta_x * ROTATE_AMT)
            * Quaternion::from_axis_angle(right, inputs.mouse_delta_y * ROTATE_AMT)
            * rotation;

        cam_rotated = true;
    }

    if cam_moved {
        cam.position = cam.position + cam.orientation.rotate_vec(movement_vec);
    }

    if cam_rotated {
        cam.orientation = rotation * cam.orientation;
    }
}
