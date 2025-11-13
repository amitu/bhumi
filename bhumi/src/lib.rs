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
    
    // Rotation modes (IJKL cluster with shift modifier)
    SteerPitchUp,    // I - Nose up + change travel direction
    SteerPitchDown,  // K - Nose down + change travel direction
    SteerYawLeft,    // J - Turn left + change travel direction
    SteerYawRight,   // L - Turn right + change travel direction
    SteerRollLeft,   // U - Bank left + change travel direction
    SteerRollRight,  // O - Bank right + change travel direction
    
    LookPitchUp,     // Shift+I - Look up only
    LookPitchDown,   // Shift+K - Look down only
    LookYawLeft,     // Shift+J - Look left only
    LookYawRight,    // Shift+L - Look right only
    LookRollLeft,    // Shift+U - Look roll left only
    LookRollRight,   // Shift+O - Look roll right only
    
    ResetLookDirection, // Release shift - reset view to travel direction
    
    // Stopping modes
    GentleStop,      // 9 - Gradual passenger-friendly stop
    EmergencyBrake,  // Shift+9 - Quick emergency brake
    
    // Utility
    CameraMode(CameraMode),
    Reset,
    Exit,
}

/// Camera viewing modes
#[derive(Debug, Clone, Copy)]
pub enum CameraMode {
    FirstPerson, // Inside drone cockpit
    ThirdPerson, // Behind/above drone
    FreeCam,     // User-controlled camera
}
