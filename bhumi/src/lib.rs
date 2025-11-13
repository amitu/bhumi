// Bhumi 3D Engine - Core library
// Provides 3D graphics engine with modular renderer backends

pub mod pixel_buffer;
pub mod camera;
pub mod physics;
pub mod renderer;

pub use pixel_buffer::*;
pub use camera::*;
pub use physics::*;
pub use renderer::*;

use std::io::Result;

/// Trait for backend renderers (terminal, GPU, etc)
pub trait PixelRenderer {
    fn new() -> Self;
    fn render_frame(&mut self, buffer: &PixelBuffer) -> Result<()>;
    fn handle_input(&mut self) -> Vec<InputEvent>;
    fn should_exit(&self) -> bool;
}

/// Input events from user interaction
#[derive(Debug, Clone)]
pub enum InputEvent {
    ThrustUp,
    ThrustDown,
    ThrustLeft,
    ThrustRight,
    ThrustForward,
    ThrustBackward,
    CameraMode(CameraMode),
    ToggleRenderMode,
    Reset,
    Exit,
}

/// Terminal rendering modes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RenderMode {
    Braille,    // 2×4 pixels per char (160×120 effective resolution)
    Block,      // Block characters with color
    Ascii,      // Original ASCII brightness conversion
}

/// Camera viewing modes
#[derive(Debug, Clone, Copy)]
pub enum CameraMode {
    FirstPerson,  // Inside drone cockpit
    ThirdPerson,  // Behind/above drone
    FreeCam,      // User-controlled camera
}