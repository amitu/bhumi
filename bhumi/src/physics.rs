use rapier3d::prelude::*;

/// Wrapper around Rapier physics world
/// - Static room (floor + 4 walls + ceiling)  
/// - Dynamic drone body
/// - Physics simulation in SI units (meters, seconds)
pub struct PhysicsWorld {
    gravity: Vector<f32>,
    integration_parameters: IntegrationParameters,
    pipeline: PhysicsPipeline,
    island_manager: IslandManager,
    broad_phase: BroadPhaseBvh,
    narrow_phase: NarrowPhase,
    bodies: RigidBodySet,
    colliders: ColliderSet,
    impulse_joints: ImpulseJointSet,
    multibody_joints: MultibodyJointSet,
    ccd_solver: CCDSolver,
    // handles
    drone_handle: RigidBodyHandle,
}

impl PhysicsWorld {
    /// Create new physics world with room and drone
    pub fn new() -> Self {
        let gravity = Vector::new(0.0, 0.0, 0.0); // zero gravity for free exploration
        let mut bodies = RigidBodySet::new();
        let mut colliders = ColliderSet::new();

        // Create drone (dynamic sphere) - start in front of cube with no motion
        let mut rb = RigidBodyBuilder::dynamic()
            .translation(vector![0.0, 0.0, -3.0]) // start 3m in front of cube
            .linvel(vector![0.0, 0.0, 0.0]) // no initial velocity - motion only via controls
            .build();
        // set high damping for responsive control and easy stopping
        rb.set_linear_damping(0.9);  // Higher damping for quicker stops
        rb.set_angular_damping(0.9);
        let drone_handle = bodies.insert(rb);
        let drone_collider = ColliderBuilder::ball(0.35)
            .restitution(0.3)
            .friction(0.7)
            .build();
        colliders.insert_with_parent(drone_collider, drone_handle, &mut bodies);

        // Simple cube made of wireframe (no solid colliders for now)
        // Cube size: 2x2x2 meters, centered at origin

        Self {
            gravity,
            integration_parameters: IntegrationParameters { dt: 1.0 / 60.0, ..Default::default() },
            pipeline: PhysicsPipeline::new(),
            island_manager: IslandManager::new(),
            broad_phase: BroadPhaseBvh::new(),
            narrow_phase: NarrowPhase::new(),
            bodies,
            colliders,
            impulse_joints: ImpulseJointSet::new(),
            multibody_joints: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(),
            drone_handle,
        }
    }

    /// Step the physics world by `dt` seconds
    /// `force_world` is a Vector<f32> in world coords applied to the drone this step (e.g. thrust)
    /// Returns the drone position as [x,y,z]
    pub fn step(&mut self, dt: f32, force_world: Vector<f32>) -> [f32; 3] {
        // set the integration dt to the provided dt
        self.integration_parameters.dt = dt.max(1.0 / 240.0); // clamp small dt
        // apply force to drone
        if let Some(rb) = self.bodies.get_mut(self.drone_handle) {
            rb.add_force(force_world, true);
        }

        // step the physics pipeline
        self.pipeline.step(
            &self.gravity,
            &self.integration_parameters,
            &mut self.island_manager,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.bodies,
            &mut self.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            &mut self.ccd_solver,
            &(),
            &(),
        );

        // read drone position
        if let Some(rb) = self.bodies.get(self.drone_handle) {
            let t = rb.translation();
            [t.x, t.y, t.z]
        } else {
            [0.0, 0.0, 0.0]
        }
    }

    /// Get drone position without stepping physics
    pub fn get_drone_position(&self) -> [f32; 3] {
        if let Some(rb) = self.bodies.get(self.drone_handle) {
            let t = rb.translation();
            [t.x, t.y, t.z]
        } else {
            [0.0, 0.0, 0.0]
        }
    }

    /// Get drone velocity
    pub fn get_drone_velocity(&self) -> [f32; 3] {
        if let Some(rb) = self.bodies.get(self.drone_handle) {
            let v = rb.linvel();
            [v.x, v.y, v.z]
        } else {
            [0.0, 0.0, 0.0]
        }
    }

    /// Reset drone to starting position and stop all motion
    pub fn reset_drone(&mut self) {
        if let Some(rb) = self.bodies.get_mut(self.drone_handle) {
            rb.set_translation(vector![0.0, 0.0, -3.0], true); // 3m in front of cube
            rb.set_linvel(vector![0.0, 0.0, 0.0], true);       // no initial velocity
            rb.set_angvel(vector![0.0, 0.0, 0.0], true);       // no rotation
        }
    }

    /// Stop drone motion without changing position
    pub fn stop_drone(&mut self) {
        if let Some(rb) = self.bodies.get_mut(self.drone_handle) {
            rb.set_linvel(vector![0.0, 0.0, 0.0], true);       // stop all velocity
            rb.set_angvel(vector![0.0, 0.0, 0.0], true);       // stop rotation
        }
    }
}