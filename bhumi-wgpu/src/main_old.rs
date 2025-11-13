use bhumi::{PixelRenderer, PixelBuffer, Renderer, InputEvent, CameraMode};
use log::info;
use pixels::{Pixels, SurfaceTexture};
use std::time::Instant;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId, Fullscreen},
};

const BUFFER_WIDTH: u32 = 320;
const BUFFER_HEIGHT: u32 = 240;

struct WgpuRenderer {
    window: Option<Window>,
    pixels: Option<Pixels>,
    bhumi_renderer: Renderer,
    last_frame: Instant,
    keys_pressed: std::collections::HashSet<KeyCode>,
    is_fullscreen: bool,
}

impl PixelRenderer for WgpuRenderer {
    fn new() -> Self {
        Self {
            window: None,
            pixels: None,
            bhumi_renderer: Renderer::new(),
            last_frame: Instant::now(),
            keys_pressed: std::collections::HashSet::new(),
            is_fullscreen: false,
        }
    }

    fn render_frame(&mut self, buffer: &PixelBuffer) -> Result<(), std::io::Error> {
        if let Some(pixels) = &mut self.pixels {
            let frame = pixels.frame_mut();
            
            // Copy our 320×240 RGBA buffer to pixels frame
            for (i, pixel) in buffer.pixels.iter().enumerate() {
                let offset = i * 4;
                if offset + 3 < frame.len() {
                    frame[offset] = pixel[0];     // R
                    frame[offset + 1] = pixel[1]; // G
                    frame[offset + 2] = pixel[2]; // B
                    frame[offset + 3] = pixel[3]; // A
                }
            }
            
            if let Err(err) = pixels.render() {
                log::error!("pixels.render() failed: {}", err);
            }
        }
        Ok(())
    }

    fn handle_input(&mut self) -> Vec<InputEvent> {
        let mut events = Vec::new();
        
        // Convert held keys to input events
        for key in &self.keys_pressed {
            match key {
                // Translation (WASD)
                KeyCode::KeyW => events.push(InputEvent::ThrustForward),
                KeyCode::KeyS => events.push(InputEvent::ThrustBackward),
                KeyCode::KeyA => events.push(InputEvent::ThrustLeft),
                KeyCode::KeyD => events.push(InputEvent::ThrustRight),
                KeyCode::Space => events.push(InputEvent::ThrustUp),
                KeyCode::KeyC => events.push(InputEvent::ThrustDown),
                
                // Rotation (IJKL) - steering mode for now
                KeyCode::KeyI => events.push(InputEvent::SteerPitchUp),
                KeyCode::KeyK => events.push(InputEvent::SteerPitchDown),
                KeyCode::KeyJ => events.push(InputEvent::SteerYawLeft),
                KeyCode::KeyL => events.push(InputEvent::SteerYawRight),
                KeyCode::KeyU => events.push(InputEvent::SteerRollLeft),
                KeyCode::KeyO => events.push(InputEvent::SteerRollRight),
                
                _ => {}
            }
        }
        
        events
    }

    fn should_exit(&self) -> bool {
        false // Handled by winit event loop
    }
}

impl WgpuRenderer {
    fn create_window(&mut self, event_loop: &ActiveEventLoop) {
        let monitor = event_loop.primary_monitor().unwrap();
        let monitor_size = monitor.size();
        
        // Calculate adaptive scaling
        let scale_x = monitor_size.width / BUFFER_WIDTH;
        let scale_y = monitor_size.height / BUFFER_HEIGHT;
        let scale = std::cmp::min(scale_x, scale_y).max(2); // At least 2x scaling
        
        let window_size = LogicalSize::new(
            BUFFER_WIDTH * scale,
            BUFFER_HEIGHT * scale,
        );
        
        let window_attributes = Window::default_attributes()
            .with_title("Bhumi 3D - GPU Accelerated")
            .with_inner_size(window_size)
            .with_min_inner_size(LogicalSize::new(BUFFER_WIDTH * 2, BUFFER_HEIGHT * 2));
        
        let window = event_loop.create_window(window_attributes).unwrap();
        
        // Create pixels renderer
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        let pixels = Pixels::new(BUFFER_WIDTH, BUFFER_HEIGHT, surface_texture).unwrap();
        
        info!("Created window: {}×{} (scale: {}x)", window_size.width, window_size.height, scale);
        
        self.window = Some(window);
        self.pixels = Some(pixels);
    }
    
    fn toggle_fullscreen(&mut self) {
        if let Some(window) = &self.window {
            self.is_fullscreen = !self.is_fullscreen;
            
            if self.is_fullscreen {
                window.set_fullscreen(Some(Fullscreen::Borderless(None)));
            } else {
                window.set_fullscreen(None);
            }
        }
    }
}

impl ApplicationHandler for WgpuRenderer {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.create_window(event_loop);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                info!("Window close requested");
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
                        // Handle special keys
                        match key_code {
                            KeyCode::Escape => event_loop.exit(),
                            KeyCode::F11 => self.toggle_fullscreen(),
                            KeyCode::Digit0 => {
                                self.bhumi_renderer.update(0.016, &[InputEvent::Reset]);
                            },
                            KeyCode::Digit9 => {
                                self.bhumi_renderer.update(0.016, &[InputEvent::GentleStop]);
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

                // Update 3D world
                let input_events = self.handle_input();
                self.bhumi_renderer.update(dt, &input_events);
                self.bhumi_renderer.render();

                // Render to GPU
                self.render_frame(&self.bhumi_renderer.buffer).ok();

                // Request next frame
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::Resized(new_size) => {
                if let Some(pixels) = &mut self.pixels {
                    if let Err(err) = pixels.resize_surface(new_size.width, new_size.height) {
                        log::error!("pixels.resize_surface() failed: {}", err);
                    }
                }
            }
            _ => {}
        }
    }
}

fn main() {
    env_logger::init();
    
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    
    let mut app = WgpuRenderer::new();
    
    info!("Starting Bhumi 3D GPU renderer");
    info!("Controls: WASD=move, IJKL=rotate, F11=fullscreen, ESC=quit");
    
    event_loop.run_app(&mut app).unwrap();
}