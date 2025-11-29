// Simple bhumi-gui using pixels crate (no complex wgpu setup)
use bhumi::{Renderer, InputEvent};
use log::info;
use pixels::{Pixels, SurfaceTexture};
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
    // Rendering
    window: Option<Window>,
    
    // Use the same core renderer as terminal version
    core_renderer: Renderer,
    
    // Input and timing
    keys_pressed: HashSet<KeyCode>,
    last_frame: Instant,
    is_fullscreen: bool,
}

impl BhumiGpuApp {
    fn new() -> Self {
        Self {
            window: None,
            core_renderer: Renderer::new(),
            keys_pressed: HashSet::new(),
            last_frame: Instant::now(),
            is_fullscreen: false,
        }
    }
    
    // Removed init_pixels since we create fresh each frame
    
    fn handle_input(&mut self) -> Vec<InputEvent> {
        let mut events = Vec::new();
        
        // Convert held keys to InputEvents (same as terminal version)
        for key in &self.keys_pressed {
            match key {
                // Translation (WASD) - using correct winit key codes
                KeyCode::KeyW => {
                    info!("W pressed - thrust forward");
                    events.push(InputEvent::ThrustForward);
                },
                KeyCode::KeyS => {
                    info!("S pressed - thrust backward");
                    events.push(InputEvent::ThrustBackward);
                }, 
                KeyCode::KeyA => {
                    info!("A pressed - thrust left");
                    events.push(InputEvent::ThrustLeft);
                },
                KeyCode::KeyD => {
                    info!("D pressed - thrust right");
                    events.push(InputEvent::ThrustRight);
                },
                KeyCode::Space => {
                    info!("SPACE pressed - thrust up");
                    events.push(InputEvent::ThrustUp);
                },
                KeyCode::KeyC => {
                    info!("C pressed - thrust down");
                    events.push(InputEvent::ThrustDown);
                },
                
                // Rotation (IJKL)
                KeyCode::KeyI => {
                    info!("I pressed - pitch up");
                    events.push(InputEvent::SteerPitchUp);
                },
                KeyCode::KeyK => {
                    info!("K pressed - pitch down");
                    events.push(InputEvent::SteerPitchDown);
                },
                KeyCode::KeyJ => {
                    info!("J pressed - yaw left");
                    events.push(InputEvent::SteerYawLeft);
                },
                KeyCode::KeyL => {
                    info!("L pressed - yaw right");
                    events.push(InputEvent::SteerYawRight);
                },
                KeyCode::KeyU => events.push(InputEvent::SteerRollLeft),
                KeyCode::KeyO => events.push(InputEvent::SteerRollRight),
                _ => {}
            }
        }
        
        events
    }
    
    fn render(&mut self) {
        // Use bhumi core renderer (same as terminal)
        self.core_renderer.render();
        
        // Create pixels renderer fresh each frame to avoid lifetime issues
        if let Some(window) = &self.window {
            let window_size = window.inner_size();
            let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, window);
            
            if let Ok(mut pixels) = Pixels::new(RENDER_WIDTH, RENDER_HEIGHT, surface_texture) {
                let frame = pixels.frame_mut();
                
                // Copy bhumi core's 320×240 RGBA buffer to pixels
                for (i, pixel) in self.core_renderer.buffer.pixels.iter().enumerate() {
                    let offset = i * 4;
                    if offset + 3 < frame.len() {
                        frame[offset] = pixel[0];     // R
                        frame[offset + 1] = pixel[1]; // G
                        frame[offset + 2] = pixel[2]; // B
                        frame[offset + 3] = pixel[3]; // A
                    }
                }
                
                if let Err(e) = pixels.render() {
                    log::error!("Render failed: {}", e);
                }
            }
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
        
        // Pixels renderer will be created fresh each frame
        
        info!("🎮 Bhumi GUI: {}×{} ({}x scale) on {}×{} monitor", 
            window_size.width, window_size.height, scale, monitor_size.width, monitor_size.height);
        info!("🎯 Controls: WASD=fly (red square moves), IJKL=rotate, Q=quit, F11=fullscreen");
        
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
                                self.core_renderer.update(0.016, &[InputEvent::Reset]);
                            },
                            KeyCode::Digit9 => {
                                info!("🛑 Gentle stop");
                                self.core_renderer.update(0.016, &[InputEvent::GentleStop]);
                            },
                            _ => {
                                info!("🔍 Key pressed: {:?}", key_code);
                                self.keys_pressed.insert(key_code);
                                
                                // Also process input immediately for single keypresses
                                let immediate_events = match key_code {
                                    KeyCode::KeyW => vec![InputEvent::ThrustForward],
                                    KeyCode::KeyS => vec![InputEvent::ThrustBackward],
                                    KeyCode::KeyA => vec![InputEvent::ThrustLeft],
                                    KeyCode::KeyD => vec![InputEvent::ThrustRight],
                                    KeyCode::Space => vec![InputEvent::ThrustUp],
                                    KeyCode::KeyC => vec![InputEvent::ThrustDown],
                                    KeyCode::KeyI => vec![InputEvent::SteerPitchUp],
                                    KeyCode::KeyK => vec![InputEvent::SteerPitchDown],
                                    KeyCode::KeyJ => vec![InputEvent::SteerYawLeft],
                                    KeyCode::KeyL => vec![InputEvent::SteerYawRight],
                                    _ => vec![],
                                };
                                
                                if !immediate_events.is_empty() {
                                    info!("🎮 Processing immediate input: {:?}", immediate_events);
                                    self.core_renderer.update(0.016, &immediate_events);
                                }
                            }
                        }
                    }
                    ElementState::Released => {
                        info!("🔍 Key released: {:?}", key_code);
                        self.keys_pressed.remove(&key_code);
                    }
                }
            }
            
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = (now - self.last_frame).as_secs_f32();
                self.last_frame = now;
                
                // Update simulation (same as terminal version)
                let input_events = self.handle_input();
                self.core_renderer.update(dt, &input_events);
                
                // Debug: log drone position occasionally
                static mut FRAME_COUNT: u32 = 0;
                unsafe {
                    FRAME_COUNT += 1;
                    if FRAME_COUNT % 60 == 0 {
                        let pos = self.core_renderer.get_drone_position();
                        let vel = self.core_renderer.get_drone_velocity();
                        info!("🚁 Drone: pos=({:.2},{:.2},{:.2}) vel=({:.2},{:.2},{:.2})", 
                            pos[0], pos[1], pos[2], vel[0], vel[1], vel[2]);
                    }
                }
                
                self.render();
                
                // Request next frame immediately for continuous rendering
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
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
    // Use continuous polling like terminal version for debugging
    event_loop.set_control_flow(ControlFlow::Poll);
    
    let mut app = BhumiGpuApp::new();
    
    info!("🚀 Bhumi GUI starting with pixels renderer...");
    
    if let Err(e) = event_loop.run_app(&mut app) {
        log::error!("Event loop error: {}", e);
    }
}