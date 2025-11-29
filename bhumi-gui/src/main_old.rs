use bhumi::{PixelRenderer, PixelBuffer, Renderer, InputEvent};
use log::info;
use pixels::{Pixels, SurfaceTexture, PixelsBuilder};
use std::collections::HashSet;
use std::time::Instant;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey, ModifiersState},
    window::{Window, WindowId, Fullscreen},
};

const BUFFER_WIDTH: u32 = 320;
const BUFFER_HEIGHT: u32 = 240;

/// GPU renderer implementing the PixelRenderer trait (matching bhumi-terminal pattern)
struct GpuRenderer {
    should_exit: bool,
    keys_pressed: HashSet<KeyCode>,
    shift_pressed: bool,
    last_frame: Instant,
}

impl PixelRenderer for GpuRenderer {
    fn new() -> Self {
        Self {
            should_exit: false,
            keys_pressed: HashSet::new(),
            shift_pressed: false,
            last_frame: Instant::now(),
        }
    }

    fn render_frame(&mut self, buffer: &PixelBuffer) -> Result<(), std::io::Error> {
        // This will be called from the GPU app context
        Ok(())
    }

    fn handle_input(&mut self) -> Vec<InputEvent> {
        let mut events = Vec::new();
        
        // Convert held keys to input events (same as terminal)
        for key in &self.keys_pressed {
            match key {
                // Translation controls (WASD cluster - left hand)
                KeyCode::KeyW => events.push(InputEvent::ThrustForward),
                KeyCode::KeyS => events.push(InputEvent::ThrustBackward),
                KeyCode::KeyA => events.push(InputEvent::ThrustLeft),
                KeyCode::KeyD => events.push(InputEvent::ThrustRight),
                KeyCode::Space => events.push(InputEvent::ThrustUp),
                KeyCode::KeyC => events.push(InputEvent::ThrustDown),
                
                // Rotation controls (IJKL cluster) - behavior depends on shift
                KeyCode::KeyI => {
                    if self.shift_pressed {
                        events.push(InputEvent::LookPitchUp);
                    } else {
                        events.push(InputEvent::SteerPitchUp);
                    }
                },
                KeyCode::KeyK => {
                    if self.shift_pressed {
                        events.push(InputEvent::LookPitchDown);
                    } else {
                        events.push(InputEvent::SteerPitchDown);
                    }
                },
                KeyCode::KeyJ => {
                    if self.shift_pressed {
                        events.push(InputEvent::LookYawLeft);
                    } else {
                        events.push(InputEvent::SteerYawLeft);
                    }
                },
                KeyCode::KeyL => {
                    if self.shift_pressed {
                        events.push(InputEvent::LookYawRight);
                    } else {
                        events.push(InputEvent::SteerYawRight);
                    }
                },
                KeyCode::KeyU => {
                    if self.shift_pressed {
                        events.push(InputEvent::LookRollLeft);
                    } else {
                        events.push(InputEvent::SteerRollLeft);
                    }
                },
                KeyCode::KeyO => {
                    if self.shift_pressed {
                        events.push(InputEvent::LookRollRight);
                    } else {
                        events.push(InputEvent::SteerRollRight);
                    }
                },
                _ => {}
            }
        }
        
        events
    }

    fn should_exit(&self) -> bool {
        self.should_exit
    }
}

/// Main GPU application  
struct GpuApp {
    window: Option<Window>,
    gpu_renderer: GpuRenderer,
    core_renderer: Renderer,
    is_fullscreen: bool,
}

impl GpuApp {
    fn new() -> Self {
        Self {
            window: None,
            gpu_renderer: GpuRenderer::new(),
            core_renderer: Renderer::new(),
            is_fullscreen: false,
        }
    }
    
    fn create_window(&mut self, event_loop: &ActiveEventLoop) {
        let monitor = event_loop.primary_monitor().unwrap();
        let monitor_size = monitor.size();
        
        // Calculate adaptive scaling for high-res displays
        let scale_x = monitor_size.width / BUFFER_WIDTH;
        let scale_y = monitor_size.height / BUFFER_HEIGHT;
        let scale = std::cmp::min(scale_x, scale_y).max(2); // At least 2x scaling
        
        let window_size = LogicalSize::new(
            BUFFER_WIDTH * scale,
            BUFFER_HEIGHT * scale,
        );
        
        let window_attributes = Window::default_attributes()
            .with_title("Bhumi 3D - GPU Accelerated Flight")
            .with_inner_size(window_size)
            .with_min_inner_size(LogicalSize::new(BUFFER_WIDTH * 2, BUFFER_HEIGHT * 2));
        
        let window = event_loop.create_window(window_attributes).unwrap();
        let window_size = window.inner_size();
        
        info!("GPU Window: {}×{} ({}x scale) | Monitor: {}×{}", 
            window_size.width, window_size.height, scale, monitor_size.width, monitor_size.height);
        
        self.window = Some(window);
    }
    
    fn toggle_fullscreen(&mut self) {
        if let Some(window) = &self.window {
            self.is_fullscreen = !self.is_fullscreen;
            
            if self.is_fullscreen {
                window.set_fullscreen(Some(Fullscreen::Borderless(None)));
                info!("Entering fullscreen mode");
            } else {
                window.set_fullscreen(None);
                info!("Exiting fullscreen mode");
            }
        }
    }
    
    fn render_to_gpu(&mut self) {
        // For now, just create a simple colored window to test
        if let Some(window) = &self.window {
            // Request redraw to show we're working
            window.request_redraw();
        }
    }
}

impl ApplicationHandler for GpuApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.create_window(event_loop);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                info!("Exiting bhumi-wgpu");
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
                // Track shift state
                self.gpu_renderer.shift_pressed = false; // TODO: detect shift properly
                
                match state {
                    ElementState::Pressed => {
                        match key_code {
                            KeyCode::Escape | KeyCode::KeyQ => {
                                self.gpu_renderer.should_exit = true;
                                event_loop.exit();
                            },
                            KeyCode::F11 => self.toggle_fullscreen(),
                            KeyCode::Digit0 => {
                                // Reset drone
                                self.core_renderer.update(0.016, &[InputEvent::Reset]);
                            },
                            KeyCode::Digit9 => {
                                // Gentle stop - TODO: add shift detection for emergency brake
                                self.core_renderer.update(0.016, &[InputEvent::GentleStop]);
                            },
                            _ => {
                                self.gpu_renderer.keys_pressed.insert(key_code);
                            }
                        }
                    }
                    ElementState::Released => {
                        self.gpu_renderer.keys_pressed.remove(&key_code);
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                // Simple test - just create and render pixels inline to avoid lifetime issues
                if let Some(window) = &self.window {
                    let window_size = window.inner_size();
                    let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, window);
                    
                    if let Ok(mut pixels) = Pixels::new(BUFFER_WIDTH, BUFFER_HEIGHT, surface_texture) {
                        let now = Instant::now();
                        let dt = (now - self.gpu_renderer.last_frame).as_secs_f32();
                        self.gpu_renderer.last_frame = now;

                        // Update 3D world
                        let input_events = self.gpu_renderer.handle_input();
                        self.core_renderer.update(dt, &input_events);
                        self.core_renderer.render();

                        // Copy buffer to GPU
                        let frame = pixels.frame_mut();
                        for (i, pixel) in self.core_renderer.buffer.pixels.iter().enumerate() {
                            let offset = i * 4;
                            if offset + 3 < frame.len() {
                                frame[offset] = pixel[0];     // R
                                frame[offset + 1] = pixel[1]; // G
                                frame[offset + 2] = pixel[2]; // B
                                frame[offset + 3] = pixel[3]; // A
                            }
                        }
                        
                        if let Err(err) = pixels.render() {
                            log::error!("GPU render failed: {}", err);
                        }
                    }
                    
                    window.request_redraw();
                }
            }
            WindowEvent::Resized(new_size) => {
                info!("Window resized to: {}×{}", new_size.width, new_size.height);
            }
            _ => {}
        }
    }
}

fn main() {
    env_logger::init();
    
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    
    let mut app = GpuApp::new();
    
    info!("🚀 Bhumi 3D GPU Renderer Starting");
    info!("📱 Adaptive scaling for high-res displays");
    info!("🎮 Controls: WASD=fly IJKL=rotate F11=fullscreen ESC=exit");
    
    event_loop.run_app(&mut app).unwrap();
}