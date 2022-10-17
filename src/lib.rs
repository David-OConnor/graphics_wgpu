#![allow(mixed_script_confusables)] // Theta in meshes

mod camera;
mod gui;
mod graphics;
mod system;
mod input;
pub mod lighting;
mod meshes;
mod texture;
mod types;

pub use camera::Camera;
pub use system::run;
pub use input::InputsCommanded;
pub use lighting::{LightType, Lighting, PointLight};
pub use types::{ControlScheme, Entity, InputSettings, Mesh, Scene, UiSettings, EngineUpdates};

// Re-export winit DeviceEvents for use in the API; this prevents the calling
// lib from needing to use winit as a dependency directly.
// todo: the equiv for mouse events too
pub use winit::event::{self, DeviceEvent, ElementState};
