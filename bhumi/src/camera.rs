use crate::CameraMode;
use nalgebra::{Matrix4, Point3, Vector3};

/// 3D camera for rendering world from different perspectives
pub struct Camera {
    pub mode: CameraMode,
    pub position: Point3<f32>,
    pub target: Point3<f32>,
    pub up: Vector3<f32>,
    pub fov: f32,    // Field of view in radians
    pub aspect: f32, // Width / height
    pub near: f32,   // Near clipping plane
    pub far: f32,    // Far clipping plane
}

impl Camera {
    /// Create new camera with standard settings
    pub fn new() -> Self {
        Self {
            mode: CameraMode::ThirdPerson,         // Camera behind and above drone
            position: Point3::new(0.0, 0.0, -3.0), // Will be updated relative to drone
            target: Point3::new(0.0, 0.0, 0.0),    // Will look at drone
            up: Vector3::y(),                      // Y is up
            fov: 60.0_f32.to_radians(),            // 60 degree FOV
            aspect: 320.0 / 240.0,                 // 4:3 aspect ratio
            near: 0.1,                             // 10cm near plane
            far: 100.0,                            // 100m far plane
        }
    }

    /// Update camera based on drone position and current mode
    pub fn update(&mut self, drone_pos: [f32; 3]) {
        let drone_point = Point3::new(drone_pos[0], drone_pos[1], drone_pos[2]);

        // Third-person camera: behind and above drone, looking at drone
        let offset = Vector3::new(-1.5, 1.0, -2.0); // Behind, above, and to the side
        self.position = drone_point + offset;
        self.target = drone_point; // Look at the drone
    }

    /// Get view matrix for current camera
    pub fn get_view_matrix(&self) -> Matrix4<f32> {
        Matrix4::look_at_rh(&self.position, &self.target, &self.up)
    }

    /// Get projection matrix for current camera
    pub fn get_projection_matrix(&self) -> Matrix4<f32> {
        Matrix4::new_perspective(self.aspect, self.fov, self.near, self.far)
    }

    /// Get combined view-projection matrix
    pub fn get_view_projection_matrix(&self) -> Matrix4<f32> {
        self.get_projection_matrix() * self.get_view_matrix()
    }

    /// Set camera mode
    pub fn set_mode(&mut self, mode: CameraMode) {
        self.mode = mode;
    }
}

/// Convert 3D world coordinates to 2D screen coordinates
/// Returns (x, y, depth) where x,y are in pixel coordinates and depth is normalized [0,1]
/// Returns None if point is behind camera or outside frustum
pub fn world_to_screen(
    world_pos: Point3<f32>,
    view_projection: &Matrix4<f32>,
    screen_width: u32,
    screen_height: u32,
) -> Option<(f32, f32, f32)> {
    // Transform to clip space
    let world_4d = world_pos.to_homogeneous();
    let clip_space = view_projection * world_4d;

    // Check if behind camera
    if clip_space.w <= 0.0 {
        return None;
    }

    // Perspective divide to NDC (normalized device coordinates)
    let ndc_x = clip_space.x / clip_space.w;
    let ndc_y = clip_space.y / clip_space.w;
    let ndc_z = clip_space.z / clip_space.w;

    // Temporarily disable frustum clipping to see all corners
    // if ndc_x < -1.0 || ndc_x > 1.0 || ndc_y < -1.0 || ndc_y > 1.0 || ndc_z < 0.0 || ndc_z > 1.0 {
    //     return None;
    // }

    // Convert to screen coordinates
    let screen_x = (ndc_x + 1.0) * 0.5 * screen_width as f32;
    let screen_y = (1.0 - ndc_y) * 0.5 * screen_height as f32; // Flip Y axis

    Some((screen_x, screen_y, ndc_z))
}
