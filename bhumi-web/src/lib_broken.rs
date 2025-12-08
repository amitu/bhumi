// Bhumi Web - WASM-compatible 3D flight simulator
use wasm_bindgen::prelude::*;
use web_sys::Gamepad;
use std::collections::HashSet;

// Use the exact same bhumi core as other backends
use bhumi::{Renderer, InputEvent};

// Import console.log
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

// Console log macro
macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

#[wasm_bindgen]
pub struct BhumiWeb {
    // Same core renderer as other backends
    core_renderer: Renderer,
    
    // Input state  
    keys_pressed: HashSet<String>,
    last_frame: f64,
    
    // Canvas context for rendering
    canvas: Option<web_sys::HtmlCanvasElement>,
    ctx: Option<web_sys::CanvasRenderingContext2d>,
}

#[wasm_bindgen]
impl BhumiWeb {
    #[wasm_bindgen(constructor)]
    pub fn new() -> BhumiWeb {
        // Set up panic hook for better error messages
        console_error_panic_hook::set_once();
        
        // Initialize console logging
        console_log::init_with_level(log::Level::Info).expect("Failed to init logger");
        
        console_log!("🚀 Bhumi Web initializing...");
        
        Self {
            core_renderer: Renderer::new(),
            keys_pressed: HashSet::new(),
            last_frame: 0.0,
            canvas: None,
            ctx: None,
        }
    }
    
    #[wasm_bindgen]
    pub fn init_canvas(&mut self, canvas_id: &str) -> Result<(), JsValue> {
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let canvas = document.get_element_by_id(canvas_id)
            .unwrap()
            .dyn_into::<web_sys::HtmlCanvasElement>()?;
        
        canvas.set_width(320);
        canvas.set_height(240);
        
        let ctx = canvas.get_context("2d")?
            .unwrap()
            .dyn_into::<web_sys::CanvasRenderingContext2d>()?;
        
        console_log!("✅ Canvas initialized: {}×{}", canvas.width(), canvas.height());
        
        self.canvas = Some(canvas);
        self.ctx = Some(ctx);
        Ok(())
    }
    
    #[wasm_bindgen]
    pub fn handle_key_down(&mut self, key: String) {
        self.keys_pressed.insert(key.clone());
        console_log!("🔍 Key down: {}", key);
        
        // Process input immediately
        let input_event = match key.as_str() {
            "KeyW" | "w" => Some("ThrustForward"),
            "KeyS" | "s" => Some("ThrustBackward"),
            "KeyA" | "a" => Some("ThrustLeft"),
            "KeyD" | "d" => Some("ThrustRight"),
            "Space" | " " => Some("ThrustUp"),
            "KeyC" | "c" => Some("ThrustDown"),
            "KeyI" | "i" => Some("SteerPitchUp"),
            "KeyK" | "k" => Some("SteerPitchDown"),
            "KeyJ" | "j" => Some("SteerYawLeft"),
            "KeyL" | "l" => Some("SteerYawRight"),
            _ => None,
        };
        
        if let Some(event) = input_event {
            console_log!("🎮 Input: {}", event);
            self.process_input_event(event);
        }
    }
    
    #[wasm_bindgen]
    pub fn handle_key_up(&mut self, key: String) {
        self.keys_pressed.remove(&key);
    }
    
    fn process_input_event(&mut self, event: &str) {
        // Convert web input to bhumi InputEvents
        let input_event = match event {
            "ThrustForward" => Some(InputEvent::ThrustForward),
            "ThrustBackward" => Some(InputEvent::ThrustBackward),
            "ThrustLeft" => Some(InputEvent::ThrustLeft),
            "ThrustRight" => Some(InputEvent::ThrustRight),
            "ThrustUp" => Some(InputEvent::ThrustUp),
            "ThrustDown" => Some(InputEvent::ThrustDown),
            "SteerPitchUp" => Some(InputEvent::SteerPitchUp),
            "SteerPitchDown" => Some(InputEvent::SteerPitchDown),
            "SteerYawLeft" => Some(InputEvent::SteerYawLeft),
            "SteerYawRight" => Some(InputEvent::SteerYawRight),
            _ => None,
        };
        
        if let Some(event) = input_event {
            // Use same input processing as other backends
            self.core_renderer.update(0.016, &[event]);
        }
    }
    
    #[wasm_bindgen]
    pub fn handle_gamepad(&mut self) {
        let window = web_sys::window().unwrap();
        let navigator = window.navigator();
        
        // Check for connected gamepads
        if let Ok(gamepads) = navigator.get_gamepads() {
            for i in 0..gamepads.length() {
                let gamepad_value = gamepads.get(i);
                if !gamepad_value.is_null() {
                    if let Ok(gamepad) = gamepad_value.dyn_into::<Gamepad>() {
                        self.process_gamepad(&gamepad);
                    }
                }
            }
        }
    }
    
    fn process_gamepad(&mut self, gamepad: &Gamepad) {
        let axes = gamepad.axes();
        
        // Left stick - Translation
        if axes.length() >= 2 {
            if let (Some(left_x), Some(left_y)) = (axes.get(0).as_f64(), axes.get(1).as_f64()) {
                const DEADZONE: f64 = 0.2;
                
                if left_x.abs() > DEADZONE {
                    self.thrust_force.x += (left_x as f32) * 0.3;
                }
                if left_y.abs() > DEADZONE {
                    self.thrust_force.z -= (left_y as f32) * 0.3; // Invert Y for forward/back
                }
            }
        }
        
        // Right stick - Rotation  
        if axes.length() >= 4 {
            if let (Some(right_x), Some(right_y)) = (axes.get(2).as_f64(), axes.get(3).as_f64()) {
                const DEADZONE: f64 = 0.2;
                
                if right_x.abs() > DEADZONE {
                    self.rotation_delta.y += (right_x as f32) * 0.02; // Yaw
                }
                if right_y.abs() > DEADZONE {
                    self.rotation_delta.x -= (right_y as f32) * 0.02; // Pitch (inverted)
                }
            }
        }
        
        // Buttons
        let buttons = gamepad.buttons();
        if buttons.length() > 0 {
            // A button (index 0) - gentle stop
            let button_value = buttons.get(0);
            if let Ok(button) = button_value.dyn_into::<web_sys::GamepadButton>() {
                if button.pressed() {
                    self.physics.gentle_stop();
                }
            }
        }
    }
    
    #[wasm_bindgen]
    pub fn update(&mut self, timestamp: f64) {
        let dt = if self.last_frame == 0.0 {
            0.016 // First frame
        } else {
            ((timestamp - self.last_frame) / 1000.0).min(0.033) // Cap at 30fps for stability
        };
        self.last_frame = timestamp;
        
        // Handle gamepad input
        self.handle_gamepad();
        
        // Apply rotation if any
        if self.rotation_delta.length() > 0.001 {
            self.physics.apply_rotation_delta(self.rotation_delta);
        }
        
        // Step physics
        let drone_pos = self.physics.step(dt as f32, self.thrust_force);
        let drone_rot = self.physics.get_drone_rotation();
        
        // Update camera (third-person view)
        let drone_position = Vec3::new(drone_pos[0], drone_pos[1], drone_pos[2]);
        let offset = Vec3::new(-1.5, 1.0, -2.0);
        self.camera_position = drone_position + offset;
        
        // Reset forces for next frame
        self.thrust_force = Vec3::ZERO;
        self.rotation_delta = Vec3::ZERO;
    }
    
    #[wasm_bindgen]
    pub fn render(&mut self) {
        if let Some(ctx) = &self.ctx {
            // Clear canvas
            ctx.set_fill_style(&"#141e1e".into());
            ctx.fill_rect(0.0, 0.0, 320.0, 240.0);
            
            // Draw simple test wireframe
            ctx.set_stroke_style(&"white".into());
            ctx.set_line_width(1.0);
            
            // Draw a test cube wireframe
            ctx.begin_path();
            ctx.rect(100.0, 80.0, 120.0, 80.0);
            ctx.stroke();
            
            // Draw drone as red dot
            let drone_pos = self.physics.get_drone_position();
            let screen_x = 160.0 + drone_pos[0] * 20.0; // Simple projection
            let screen_y = 120.0 - drone_pos[1] * 20.0;
            
            ctx.set_fill_style(&"red".into());
            ctx.fill_rect((screen_x - 5.0) as f64, (screen_y - 5.0) as f64, 10.0, 10.0);
            
            // Show info
            ctx.set_fill_style(&"white".into());
            ctx.set_font("12px monospace");
            let info = format!("Pos: ({:.1},{:.1},{:.1}) Vel: ({:.1},{:.1},{:.1})", 
                drone_pos[0], drone_pos[1], drone_pos[2],
                self.physics.get_drone_velocity()[0],
                self.physics.get_drone_velocity()[1], 
                self.physics.get_drone_velocity()[2]);
            ctx.fill_text(&info, 10.0, 20.0).ok();
        }
    }
    
    #[wasm_bindgen]
    pub fn reset(&mut self) {
        console_log!("🔄 Resetting drone");
        self.physics.reset_drone();
    }
    
    #[wasm_bindgen]
    pub fn stop(&mut self) {
        console_log!("🛑 Gentle stop");
        self.physics.gentle_stop();
    }
}