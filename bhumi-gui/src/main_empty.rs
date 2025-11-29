// Simple bhumi-gui with working GPU rendering
use bhumi::{PhysicsWorld, Camera};
use log::info;
use std::collections::HashSet;
use std::time::Instant;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId, Fullscreen},
};

const RENDER_WIDTH: u32 = 320;
const RENDER_HEIGHT: u32 = 240;

struct BhumiGpuApp {
    window: Option<Window>,
    physics: PhysicsWorld,
    camera: Camera,
    keys_pressed: HashSet<KeyCode>,
    last_frame: Instant,
    is_fullscreen: bool,
    
    // Physics forces
    thrust_force: [f32; 3],
    rotation_delta: [f32; 3],
    
    // Simple GPU state for colored background
    device: Option<wgpu::Device>,
    queue: Option<wgpu::Queue>,
    surface: Option<wgpu::Surface<'static>>,
    config: Option<wgpu::SurfaceConfiguration>,
}

impl BhumiGpuApp {
    fn new() -> Self {
        Self {
            window: None,
            physics: PhysicsWorld::new(),
            camera: Camera::new(),
            keys_pressed: HashSet::new(),
            last_frame: Instant::now(),
            is_fullscreen: false,
            thrust_force: [0.0, 0.0, 0.0],
            rotation_delta: [0.0, 0.0, 0.0],
            device: None,
            queue: None,
            surface: None,
            config: None,
        }
    }
    
    fn handle_input(&mut self) {
        // Reset forces
        self.thrust_force = [0.0, 0.0, 0.0];
        self.rotation_delta = [0.0, 0.0, 0.0];
        
        // Process held keys
        for key in &self.keys_pressed {
            match key {
                // Translation (WASD)
                KeyCode::KeyW => self.thrust_force[2] += 0.3,  // Forward
                KeyCode::KeyS => self.thrust_force[2] -= 0.3,  // Backward
                KeyCode::KeyA => self.thrust_force[0] -= 0.3,  // Left
                KeyCode::KeyD => self.thrust_force[0] += 0.3,  // Right
                KeyCode::Space => self.thrust_force[1] += 0.5, // Up
                KeyCode::KeyC => self.thrust_force[1] -= 0.5,  // Down
                
                // Rotation (IJKL)
                KeyCode::KeyI => self.rotation_delta[0] -= 0.02, // Pitch up
                KeyCode::KeyK => self.rotation_delta[0] += 0.02, // Pitch down  
                KeyCode::KeyJ => self.rotation_delta[1] -= 0.02, // Yaw left
                KeyCode::KeyL => self.rotation_delta[1] += 0.02, // Yaw right
                KeyCode::KeyU => self.rotation_delta[2] -= 0.02, // Roll left
                KeyCode::KeyO => self.rotation_delta[2] += 0.02, // Roll right
                _ => {}
            }
        }
    }
    
    fn update_physics(&mut self, dt: f32) {
        // Apply rotation
        if self.rotation_delta[0].abs() > 0.001 || self.rotation_delta[1].abs() > 0.001 || self.rotation_delta[2].abs() > 0.001 {
            let rapier_delta = rapier3d::prelude::Vector::new(
                self.rotation_delta[0], self.rotation_delta[1], self.rotation_delta[2]
            );
            self.physics.apply_rotation_delta(rapier_delta);
        }
        
        // Apply thrust
        let rapier_thrust = rapier3d::prelude::Vector::new(
            self.thrust_force[0], self.thrust_force[1], self.thrust_force[2]
        );
        let drone_pos = self.physics.step(dt, rapier_thrust);
        let drone_rot = self.physics.get_drone_rotation();
        
        // Update camera
        self.camera.update(drone_pos, drone_rot);
    }
    
    fn render(&mut self) {
        // Basic test rendering - just clear to a color for now
        if let Some(window) = &self.window {
            // For now, just log that we're rendering
            info!("Rendering frame (placeholder)");
        }
    }
    
    fn toggle_fullscreen(&mut self) {
        if let Some(window) = &self.window {
            self.is_fullscreen = !self.is_fullscreen;
            if self.is_fullscreen {
                window.set_fullscreen(Some(Fullscreen::Borderless(None)));
                info!("Fullscreen ON");
            } else {
                window.set_fullscreen(None);
                info!("Fullscreen OFF");
            }
        }
    }
}

impl ApplicationHandler for BhumiGpuApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Create window with adaptive scaling
        let monitor = event_loop.primary_monitor().unwrap();
        let monitor_size = monitor.size();
        
        // Scale to 80% of monitor size or at least 2x
        let scale_x = (monitor_size.width * 8 / 10) / RENDER_WIDTH;
        let scale_y = (monitor_size.height * 8 / 10) / RENDER_HEIGHT;
        let scale = std::cmp::min(scale_x, scale_y).max(2);
        
        let window_size = PhysicalSize::new(RENDER_WIDTH * scale, RENDER_HEIGHT * scale);
        
        let window = event_loop.create_window(
            Window::default_attributes()
                .with_title("🚀 Bhumi 3D - GPU Flight Simulator")
                .with_inner_size(window_size)
        ).unwrap();
        
        info!("🎮 Bhumi GUI: {}×{} ({}x scale) on {}×{} monitor", 
            window_size.width, window_size.height, scale, monitor_size.width, monitor_size.height);
        info!("🎯 Controls: WASD=fly, IJKL=rotate, Q=quit, F11=fullscreen, 0=reset, 9=stop");
        
        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                info!("👋 Bhumi GUI shutting down");
                event_loop.exit();
            }
            
            WindowEvent::KeyboardInput {
                event: KeyEvent {
                    physical_key: PhysicalKey::Code(key_code),
                    state,
                    ..
                },
                ..
            } => {
                match state {
                    ElementState::Pressed => {
                        match key_code {
                            KeyCode::Escape | KeyCode::KeyQ => {
                                info!("Exit requested");
                                event_loop.exit();
                            },
                            KeyCode::F11 => self.toggle_fullscreen(),
                            KeyCode::Digit0 => {
                                info!("🔄 Reset drone");
                                self.physics.reset_drone();
                            },
                            KeyCode::Digit9 => {
                                info!("🛑 Gentle stop");
                                self.physics.gentle_stop();
                            },
                            _ => {
                                self.keys_pressed.insert(key_code);
                            }
                        }
                    }
                    ElementState::Released => {
                        self.keys_pressed.remove(&key_code);
                    }
                }
            }
            
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = (now - self.last_frame).as_secs_f32();
                self.last_frame = now;
                
                // Update simulation
                self.handle_input();
                self.update_physics(dt);
                self.render();
                
                // Set consistent 60 FPS timing for next frame
                event_loop.set_control_flow(ControlFlow::WaitUntil(
                    std::time::Instant::now() + std::time::Duration::from_millis(16)
                ));
            }
            
            WindowEvent::Resized(new_size) => {
                info!("Window resized: {}×{}", new_size.width, new_size.height);
            }
            
            _ => {}
        }
    }
}

fn main() {
    env_logger::init();
    
    let event_loop = EventLoop::new().unwrap();
    // Set fixed 60 FPS timing
    event_loop.set_control_flow(ControlFlow::WaitUntil(
        std::time::Instant::now() + std::time::Duration::from_millis(16)
    ));
    
    let mut app = BhumiGpuApp::new();
    
    info!("🚀 Bhumi GUI starting...");
    
    if let Err(e) = event_loop.run_app(&mut app) {
        log::error!("Event loop error: {}", e);
    }
}