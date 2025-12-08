// Simple web-compatible physics (no rapier3d dependency)
use glam::{Vec3, Quat};

pub struct WebPhysicsWorld {
    // Drone state
    position: Vec3,
    velocity: Vec3,
    rotation: Quat,
    angular_velocity: Vec3,
    
    // Physics constants
    linear_damping: f32,
    angular_damping: f32,
}

impl WebPhysicsWorld {
    pub fn new() -> Self {
        Self {
            position: Vec3::new(0.0, 0.0, -3.0), // Start in front of cube
            velocity: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            angular_velocity: Vec3::ZERO,
            linear_damping: 0.9,
            angular_damping: 0.9,
        }
    }
    
    pub fn step(&mut self, dt: f32, force: Vec3) -> [f32; 3] {
        // Apply force
        let acceleration = force;
        self.velocity += acceleration * dt;
        
        // Apply damping
        self.velocity *= self.linear_damping;
        self.angular_velocity *= self.angular_damping;
        
        // Update position
        self.position += self.velocity * dt;
        
        // Update rotation
        if self.angular_velocity.length() > 0.001 {
            let rotation_delta = Quat::from_scaled_axis(self.angular_velocity * dt);
            self.rotation = rotation_delta * self.rotation;
            self.rotation = self.rotation.normalize();
        }
        
        [self.position.x, self.position.y, self.position.z]
    }
    
    pub fn apply_rotation_delta(&mut self, rotation_delta: Vec3) {
        let delta_quat = Quat::from_euler(
            glam::EulerRot::XYZ,
            rotation_delta.x,
            rotation_delta.y, 
            rotation_delta.z
        );
        self.rotation = delta_quat * self.rotation;
        self.rotation = self.rotation.normalize();
        
        // Stop angular velocity for stability
        self.angular_velocity = Vec3::ZERO;
    }
    
    pub fn get_drone_position(&self) -> [f32; 3] {
        [self.position.x, self.position.y, self.position.z]
    }
    
    pub fn get_drone_rotation(&self) -> [f32; 4] {
        [self.rotation.x, self.rotation.y, self.rotation.z, self.rotation.w]
    }
    
    pub fn get_drone_velocity(&self) -> [f32; 3] {
        [self.velocity.x, self.velocity.y, self.velocity.z]
    }
    
    pub fn reset_drone(&mut self) {
        self.position = Vec3::new(0.0, 0.0, -3.0);
        self.velocity = Vec3::ZERO;
        self.rotation = Quat::IDENTITY;
        self.angular_velocity = Vec3::ZERO;
    }
    
    pub fn gentle_stop(&mut self) {
        self.velocity *= 0.8; // Gentle deceleration
        self.angular_velocity *= 0.8;
    }
    
    pub fn emergency_brake(&mut self) {
        self.velocity *= 0.5; // Quick deceleration  
        self.angular_velocity *= 0.5;
    }
}