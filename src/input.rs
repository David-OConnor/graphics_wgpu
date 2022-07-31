// Handles keyboard and mouse input.
use std::f32::consts::TAU;

use super::lin_alg::{Mat3, Vec3};

use super::types::Camera;

// todo: remove Winit from this module if you can, and make it agnostic?
use winit::event::{DeviceEvent, ElementState};

#[derive(Copy, Clone, Debug)]
pub enum MoveDirection {
    Forward,
    Back,
    Left,
    Right,
    Up,
    Down,
}

#[derive(Default, Debug)]
pub struct ButtonState {
    pub w_pressed: bool,
    pub s_pressed: bool,
    pub a_pressed: bool,
    pub d_pressed: bool,
}

/// Find the vector representing how we move the camera, for a given direction.
/// Uses euler angles, which works for the user-controlled camera.
pub fn find_mv_vec(direction: MoveDirection, yaw: f32, pitch: f32, amount: f32) -> Vec3 {
    // Move the camera to a new position, based on where it's pointing.
    let unit_vec = match direction {
        MoveDirection::Forward => Vec3::new(0., 0., 1.),
        MoveDirection::Back => -Vec3::new(0., 0., 1.),
        // Not sure why we need to make left positive here, but it seems to be the case.
        MoveDirection::Left => Vec3::new(1., 0., 0.),
        MoveDirection::Right => -Vec3::new(1., 0., 0.),
        MoveDirection::Up => Vec3::new(0., 1., 0.),
        MoveDirection::Down => -Vec3::new(0., 1., 0.),
    };
    // Move in 2d plane only. Ie, only take yaw into account.

    // let (y_sin, y_cos) = yaw.sin_cos();
    let (y_sin, y_cos) = (yaw - TAU / 4.).sin_cos();
    #[rustfmt::skip]
    let rotation_mat = Mat3::new([
        y_cos, 0., y_sin,
         0., 1., 0.,
         -y_sin, 0., y_cos,
    ]);

    rotation_mat * (unit_vec * amount)
}

/// Handle a device event, eg input from keyboard or mouse.
pub fn handle_event(
    event: DeviceEvent,
    button_state: &mut ButtonState,
    cam: &mut Camera,
    sensitivities: &(f32, f32, f32),
    dt: f32,
) {
    let move_amount = sensitivities.0 * dt;
    let rotate_amount = sensitivities.1 * dt;
    // let zoom_amount = sensitivities.2 * dt;

    match event {
        DeviceEvent::Key(key) => match key.scancode {
            17 => {
                button_state.w_pressed = key.state == ElementState::Pressed;
            }
            31 => {
                button_state.s_pressed = key.state == ElementState::Pressed;
            }
            30 => {
                button_state.a_pressed = key.state == ElementState::Pressed;
            }
            32 => {
                button_state.d_pressed = key.state == ElementState::Pressed;
            }
            _ => (),
        },

        DeviceEvent::MouseMotion { delta } => {
            cam.yaw += delta.0 as f32 * rotate_amount;
            cam.pitch += -delta.1 as f32 * rotate_amount;

            let eps = 0.0001;

            // Clamp pitch, so you can't look past up or down.
            if cam.pitch > (TAU / 4.) - eps {
                cam.pitch = TAU / 4. - eps;
            } else if cam.pitch < -TAU / 4. + eps {
                cam.pitch = -TAU / 4. + eps;
            }
        }

        _ => {}
    }

    if button_state.w_pressed {
        cam.position += find_mv_vec(MoveDirection::Forward, cam.yaw, cam.pitch, move_amount);
    }

    if button_state.s_pressed {
        cam.position += find_mv_vec(MoveDirection::Back, cam.yaw, cam.pitch, move_amount);
    }

    if button_state.a_pressed {
        cam.position += find_mv_vec(MoveDirection::Left, cam.yaw, cam.pitch, move_amount);
    }

    if button_state.d_pressed {
        cam.position += find_mv_vec(MoveDirection::Right, cam.yaw, cam.pitch, move_amount);
    }
}
