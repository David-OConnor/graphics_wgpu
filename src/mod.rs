//! 3D rendering using Vulkan, via ash

//! https://sotrh.github.io/learn-wgpu/#

// todo: Consider breaking your 3d engine out into a separate crate.

mod init_graphics;
pub mod init_system;
mod input;
mod lin_alg;
mod texture;
mod types;
mod types_wgpu;
