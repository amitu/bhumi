// Bhumi 3D Engine - Core library
// Provides 3D graphics engine with modular renderer backends

pub mod camera;
pub mod physics;
pub mod pixel_buffer;
pub mod renderer;

pub use camera::*;
pub use physics::*;
pub use pixel_buffer::*;
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
    // Translation (WASD cluster)
    ThrustForward,   // W - Surge forward
    ThrustBackward,  // S - Surge backward
    ThrustLeft,      // A - Sway left
    ThrustRight,     // D - Sway right
    ThrustUp,        // SPACE - Heave up
    ThrustDown,      // C - Heave down
    
    // Rotation (IJKL cluster)
    PitchUp,         // I - Nose up
    PitchDown,       // K - Nose down
    YawLeft,         // J - Turn left
    YawRight,        // L - Turn right
    RollLeft,        // U - Bank left
    RollRight,       // O - Bank right
    
    // Utility
    CameraMode(CameraMode),
    Reset,
    Stop,
    Exit,
}

/// Camera viewing modes
#[derive(Debug, Clone, Copy)]
pub enum CameraMode {
    FirstPerson, // Inside drone cockpit
    ThirdPerson, // Behind/above drone
    FreeCam,     // User-controlled camera
}
