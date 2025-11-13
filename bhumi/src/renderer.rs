use crate::{Camera, CameraMode, InputEvent, PhysicsWorld, PixelBuffer, world_to_screen};
use nalgebra::Point3;
use rapier3d::prelude::Vector;

/// Core 3D renderer that manages the world, camera, and pixel buffer
pub struct Renderer {
    pub physics: PhysicsWorld,
    pub camera: Camera,
    pub buffer: PixelBuffer,
    thrust_force: Vector<f32>,
    angular_force: Vector<f32>, // For pitch/yaw/roll
}

impl Renderer {
    /// Create new renderer with default settings
    pub fn new() -> Self {
        Self {
            physics: PhysicsWorld::new(),
            camera: Camera::new(),
            buffer: PixelBuffer::new(),
            thrust_force: Vector::new(0.0, 0.0, 0.0),
            angular_force: Vector::new(0.0, 0.0, 0.0),
        }
    }

    /// Update simulation by one time step
    pub fn update(&mut self, dt: f32, input_events: &[InputEvent]) {
        // Process input events
        for event in input_events {
            match event {
                // Translation forces (WASD cluster)
                InputEvent::ThrustForward => self.thrust_force.z += 0.3,   // W - surge forward
                InputEvent::ThrustBackward => self.thrust_force.z -= 0.3,  // S - surge backward  
                InputEvent::ThrustLeft => self.thrust_force.x -= 0.3,      // A - sway left
                InputEvent::ThrustRight => self.thrust_force.x += 0.3,     // D - sway right
                InputEvent::ThrustUp => self.thrust_force.y += 0.5,        // SPACE - heave up
                InputEvent::ThrustDown => self.thrust_force.y -= 0.5,      // C - heave down
                
                // Rotational torques (IJKL cluster) - very gentle forces for subtle rotation
                InputEvent::PitchUp => self.angular_force.x -= 0.05,       // I - pitch nose up
                InputEvent::PitchDown => self.angular_force.x += 0.05,     // K - pitch nose down
                InputEvent::YawLeft => self.angular_force.y -= 0.05,       // J - yaw turn left
                InputEvent::YawRight => self.angular_force.y += 0.05,      // L - yaw turn right
                InputEvent::RollLeft => self.angular_force.z -= 0.05,      // U - roll bank left
                InputEvent::RollRight => self.angular_force.z += 0.05,     // O - roll bank right
                
                // Utility
                InputEvent::CameraMode(mode) => self.camera.set_mode(*mode),
                InputEvent::Reset => self.physics.reset_drone(),
                InputEvent::Stop => self.physics.stop_drone(),
                InputEvent::Exit => {} // Handled by renderer implementation
            }
        }

        // Step physics simulation with both linear and angular forces
        let drone_pos = self.physics.step_with_torque(dt, self.thrust_force, self.angular_force);

        // Reset forces (apply only for this frame)
        self.thrust_force = Vector::new(0.0, 0.0, 0.0);
        self.angular_force = Vector::new(0.0, 0.0, 0.0);

        // Update camera based on drone position and orientation
        let drone_rotation = self.physics.get_drone_rotation();
        self.camera.update(drone_pos, drone_rotation);
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

        // Always render drone (visible in third-person view)
        self.render_drone(&view_proj);
    }

    /// Render infinite grid of cubes
    fn render_room(&mut self, view_proj: &nalgebra::Matrix4<f32>) {
        let cube_color = [255, 255, 255, 255]; // Bright white for visibility
        
        // Get drone position to center the grid around
        let drone_pos = self.physics.get_drone_position();
        let drone_x = drone_pos[0];
        let drone_y = drone_pos[1]; 
        let drone_z = drone_pos[2];
        
        // Ultra-sparse reference grid - nearest 4 cubes only in each direction
        let cube_size = 2.0;      // 2x2x2 meter cubes 
        let cube_spacing = 15.0;  // 15 meter spacing between cubes (even more spread out)
        let grid_radius = 1;      // Only 3x3x3 total (27 cubes max)
        
        // Calculate which sparse grid cell the drone is in
        let grid_center_x = (drone_x / cube_spacing).round() as i32;
        let grid_center_y = (drone_y / cube_spacing).round() as i32;
        let grid_center_z = (drone_z / cube_spacing).round() as i32;
        
        // Render sparse cube grid as reference markers
        for gx in (grid_center_x - grid_radius)..=(grid_center_x + grid_radius) {
            for gy in (grid_center_y - grid_radius)..=(grid_center_y + grid_radius) {
                for gz in (grid_center_z - grid_radius)..=(grid_center_z + grid_radius) {
                    // World position of this reference cube (spaced 10m apart)
                    let cube_x = gx as f32 * cube_spacing;
                    let cube_y = gy as f32 * cube_spacing;  
                    let cube_z = gz as f32 * cube_spacing;
                    
                    self.render_cube_at(view_proj, cube_x, cube_y, cube_z, cube_size, cube_color);
                }
            }
        }
    }
    
    /// Render a single cube at given world position
    fn render_cube_at(&mut self, view_proj: &nalgebra::Matrix4<f32>, center_x: f32, center_y: f32, center_z: f32, size: f32, color: [u8; 4]) {
        let half_size = size / 2.0;
        
        // Cube corners relative to center
        let corners = [
            // Front face
            Point3::new(center_x - half_size, center_y - half_size, center_z - half_size), // 0
            Point3::new(center_x + half_size, center_y - half_size, center_z - half_size), // 1
            Point3::new(center_x + half_size, center_y + half_size, center_z - half_size), // 2
            Point3::new(center_x - half_size, center_y + half_size, center_z - half_size), // 3
            // Back face
            Point3::new(center_x - half_size, center_y - half_size, center_z + half_size), // 4
            Point3::new(center_x + half_size, center_y - half_size, center_z + half_size), // 5
            Point3::new(center_x + half_size, center_y + half_size, center_z + half_size), // 6
            Point3::new(center_x - half_size, center_y + half_size, center_z + half_size), // 7
        ];

        // Convert corners to screen space
        let mut screen_corners = Vec::new();
        for corner in corners.iter() {
            if let Some(screen_pos) = world_to_screen(*corner, view_proj, self.buffer.width, self.buffer.height) {
                screen_corners.push(Some((screen_pos.0 as u32, screen_pos.1 as u32)));
            } else {
                screen_corners.push(None);
            }
        }

        // Draw cube wireframe edges
        let edges = [
            // Front face
            (0, 1, color), (1, 2, color), (2, 3, color), (3, 0, color),
            // Back face  
            (4, 5, color), (5, 6, color), (6, 7, color), (7, 4, color),
            // Connecting edges (front to back)
            (0, 4, color), (1, 5, color), (2, 6, color), (3, 7, color),
        ];

        for (start_idx, end_idx, edge_color) in edges.iter() {
            if let (Some(start), Some(end)) = (screen_corners[*start_idx], screen_corners[*end_idx]) {
                self.buffer.draw_line(start.0, start.1, end.0, end.1, *edge_color);
            }
        }
    }

    /// Render the drone as a bright red dot/cross
    fn render_drone(&mut self, view_proj: &nalgebra::Matrix4<f32>) {
        let drone_pos = self.physics.get_drone_position();
        let drone_point = Point3::new(drone_pos[0], drone_pos[1], drone_pos[2]);

        if let Some(screen_pos) = world_to_screen(
            drone_point,
            view_proj,
            self.buffer.width,
            self.buffer.height,
        ) {
            let x = screen_pos.0 as u32;
            let y = screen_pos.1 as u32;
            let drone_color = [255, 0, 0, 255]; // Bright red

            // Draw drone as a larger cross for visibility
            let size = 8; // Bigger size
            if x >= size
                && y >= size
                && x + size < self.buffer.width
                && y + size < self.buffer.height
            {
                // Draw cross
                self.buffer.draw_line(x - size, y, x + size, y, drone_color);
                self.buffer.draw_line(x, y - size, x, y + size, drone_color);

                // Draw center dot for extra visibility
                self.buffer
                    .draw_rect(x.saturating_sub(2), y.saturating_sub(2), 4, 4, drone_color);
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
