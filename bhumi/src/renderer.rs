use crate::{PixelBuffer, Camera, PhysicsWorld, InputEvent, CameraMode, world_to_screen};
use nalgebra::Point3;
use rapier3d::prelude::Vector;

/// Core 3D renderer that manages the world, camera, and pixel buffer
pub struct Renderer {
    pub physics: PhysicsWorld,
    pub camera: Camera,
    pub buffer: PixelBuffer,
    thrust_force: Vector<f32>,
}

impl Renderer {
    /// Create new renderer with default settings
    pub fn new() -> Self {
        Self {
            physics: PhysicsWorld::new(),
            camera: Camera::new(),
            buffer: PixelBuffer::new(),
            thrust_force: Vector::new(0.0, 0.0, 0.0),
        }
    }

    /// Update simulation by one time step
    pub fn update(&mut self, dt: f32, input_events: &[InputEvent]) {
        // Process input events
        for event in input_events {
            match event {
                InputEvent::ThrustUp => self.thrust_force.y += 0.5,        // Space - thrust up
                InputEvent::ThrustDown => self.thrust_force.y -= 0.5,      // C - thrust down  
                InputEvent::ThrustLeft => self.thrust_force.x -= 0.3,      // A/← - thrust left
                InputEvent::ThrustRight => self.thrust_force.x += 0.3,     // D/→ - thrust right
                InputEvent::ThrustForward => self.thrust_force.z += 0.3,   // W/↑ - thrust forward
                InputEvent::ThrustBackward => self.thrust_force.z -= 0.3,  // S/↓ - thrust backward
                InputEvent::CameraMode(mode) => self.camera.set_mode(*mode),
                InputEvent::Reset => self.physics.reset_drone(),
                InputEvent::ToggleRenderMode => {}, // Handled by terminal renderer
                InputEvent::Exit => {}, // Handled by renderer implementation
            }
        }

        // Step physics simulation
        let drone_pos = self.physics.step(dt, self.thrust_force);
        
        // Reset thrust force (apply only for this frame)
        self.thrust_force = Vector::new(0.0, 0.0, 0.0);

        // Update camera based on drone position
        self.camera.update(drone_pos);
    }

    /// Render current frame to pixel buffer
    pub fn render(&mut self) {
        // Clear buffer to dark background for ASCII visibility
        self.buffer.clear([20, 20, 30, 255]); // Dark blue/black

        // Get current matrices
        let view_proj = self.camera.get_view_projection_matrix();
        
        // Remove static crosshair to see actual 3D content
        
        // Render 3D cube wireframe
        self.render_room(&view_proj);
        
        // Render drone
        self.render_drone(&view_proj);
    }

    /// Render a simple cube made of wireframe lines
    fn render_room(&mut self, view_proj: &nalgebra::Matrix4<f32>) {
        let cube_color = [255, 255, 255, 255]; // Bright white for visibility

        // Full 3D cube that drone will fly through
        let corners = [
            // Front face (z = -1) - drone approaches this
            Point3::new(-1.0, -1.0, -1.0),  // 0: front bottom-left
            Point3::new( 1.0, -1.0, -1.0),  // 1: front bottom-right  
            Point3::new( 1.0,  1.0, -1.0),  // 2: front top-right
            Point3::new(-1.0,  1.0, -1.0),  // 3: front top-left
            // Back face (z = 1) - drone exits through this
            Point3::new(-1.0, -1.0,  1.0),  // 4: back bottom-left
            Point3::new( 1.0, -1.0,  1.0),  // 5: back bottom-right
            Point3::new( 1.0,  1.0,  1.0),  // 6: back top-right
            Point3::new(-1.0,  1.0,  1.0),  // 7: back top-left
        ];

        // Convert corners to screen space - debug what's happening
        let mut screen_corners = Vec::new();
        for (i, corner) in corners.iter().enumerate() {
            if let Some(screen_pos) = world_to_screen(*corner, view_proj, self.buffer.width, self.buffer.height) {
                let x = screen_pos.0 as u32;
                let y = screen_pos.1 as u32;
                screen_corners.push(Some((x, y)));
                // Mark corner with number for debugging
                let corner_char = format!("{}", i);
                self.buffer.set_pixel(x, y, [255, 255, 0, 255]); // Yellow corner marker
            } else {
                screen_corners.push(None);
                // Corner failed projection - not visible
            }
        }

        // Draw full cube wireframe (12 edges)
        let edges = [
            // Front face
            (0, 1, cube_color), (1, 2, cube_color), (2, 3, cube_color), (3, 0, cube_color),
            // Back face
            (4, 5, cube_color), (5, 6, cube_color), (6, 7, cube_color), (7, 4, cube_color),
            // Connecting edges (front to back)
            (0, 4, cube_color), (1, 5, cube_color), (2, 6, cube_color), (3, 7, cube_color),
        ];

        for (start_idx, end_idx, color) in edges.iter() {
            if let (Some(start), Some(end)) = (screen_corners[*start_idx], screen_corners[*end_idx]) {
                self.buffer.draw_line(start.0, start.1, end.0, end.1, *color);
            }
        }
    }

    /// Render the drone (visible in third-person mode)
    fn render_drone(&mut self, view_proj: &nalgebra::Matrix4<f32>) {
        let drone_pos = self.physics.get_drone_position();
        let drone_point = Point3::new(drone_pos[0], drone_pos[1], drone_pos[2]);
        
        if let Some(screen_pos) = world_to_screen(drone_point, view_proj, self.buffer.width, self.buffer.height) {
            let x = screen_pos.0 as u32;
            let y = screen_pos.1 as u32;
            let drone_color = [255, 100, 100, 255]; // Red

            // Draw drone as a simple cross
            let size = 5;
            if x >= size && y >= size && x + size < self.buffer.width && y + size < self.buffer.height {
                self.buffer.draw_line(x - size, y, x + size, y, drone_color);
                self.buffer.draw_line(x, y - size, x, y + size, drone_color);
            }
        }
    }

    /// Get current drone position for debugging
    pub fn get_drone_position(&self) -> [f32; 3] {
        self.physics.get_drone_position()
    }

    /// Get current drone velocity for debugging
    pub fn get_drone_velocity(&self) -> [f32; 3] {
        self.physics.get_drone_velocity()
    }
}