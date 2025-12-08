// Bhumi Web - Pure renderer/IO wrapper around bhumi core
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

macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

#[wasm_bindgen]
pub struct BhumiWeb {
    // Exact same core renderer as terminal/GUI
    core_renderer: Renderer,
    
    // Web-specific rendering
    canvas: Option<web_sys::HtmlCanvasElement>,
    ctx: Option<web_sys::CanvasRenderingContext2d>,
    
    // Input state
    keys_pressed: HashSet<String>,
    last_frame: f64,
}

#[wasm_bindgen]
impl BhumiWeb {
    #[wasm_bindgen(constructor)]
    pub fn new() -> BhumiWeb {
        console_error_panic_hook::set_once();
        console_log::init_with_level(log::Level::Info).expect("Failed to init logger");
        
        console_log!("🚀 Bhumi Web with same core as terminal/GUI");
        
        Self {
            core_renderer: Renderer::new(), // SAME as terminal/GUI
            canvas: None,
            ctx: None,
            keys_pressed: HashSet::new(),
            last_frame: 0.0,
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
        
        console_log!("✅ Canvas ready for bhumi core rendering");
        
        self.canvas = Some(canvas);
        self.ctx = Some(ctx);
        Ok(())
    }
    
    #[wasm_bindgen]
    pub fn handle_key_down(&mut self, key: String) {
        self.keys_pressed.insert(key.clone());
        
        // Convert to InputEvent and send to bhumi core (same as GUI)
        let input_event = match key.as_str() {
            "KeyW" | "w" => Some(InputEvent::ThrustForward),
            "KeyS" | "s" => Some(InputEvent::ThrustBackward),
            "KeyA" | "a" => Some(InputEvent::ThrustLeft),
            "KeyD" | "d" => Some(InputEvent::ThrustRight),
            "Space" | " " => Some(InputEvent::ThrustUp),
            "KeyC" | "c" => Some(InputEvent::ThrustDown),
            "KeyI" | "i" => Some(InputEvent::SteerPitchUp),
            "KeyK" | "k" => Some(InputEvent::SteerPitchDown),
            "KeyJ" | "j" => Some(InputEvent::SteerYawLeft),
            "KeyL" | "l" => Some(InputEvent::SteerYawRight),
            _ => None,
        };
        
        if let Some(event) = input_event {
            console_log!("🎮 Input: {:?}", event);
            self.core_renderer.update(0.016, &[event]);
        }
    }
    
    #[wasm_bindgen]
    pub fn handle_key_up(&mut self, key: String) {
        self.keys_pressed.remove(&key);
    }
    
    #[wasm_bindgen]
    pub fn update(&mut self, timestamp: f64) {
        let dt = if self.last_frame == 0.0 {
            0.016
        } else {
            ((timestamp - self.last_frame) / 1000.0).min(0.033) as f32
        };
        self.last_frame = timestamp;
        
        // Update bhumi core (same as other backends)
        let input_events = Vec::new(); // Input handled in key events
        self.core_renderer.update(dt, &input_events);
        self.core_renderer.render(); // Generate 320x240 pixel buffer
    }
    
    #[wasm_bindgen]
    pub fn render(&mut self) {
        if let Some(ctx) = &self.ctx {
            // Convert bhumi core's pixel buffer to canvas (same as GUI version)
            let buffer = &self.core_renderer.buffer;
            
            // Create ImageData from pixel buffer
            let mut image_data_vec = Vec::with_capacity(buffer.pixels.len() * 4);
            for pixel in &buffer.pixels {
                image_data_vec.push(pixel[0]); // R
                image_data_vec.push(pixel[1]); // G  
                image_data_vec.push(pixel[2]); // B
                image_data_vec.push(pixel[3]); // A
            }
            
            // Create ImageData and draw to canvas
            if let Ok(image_data) = web_sys::ImageData::new_with_u8_clamped_array_and_sh(
                wasm_bindgen::Clamped(&image_data_vec), 320, 240
            ) {
                ctx.put_image_data(&image_data, 0.0, 0.0).ok();
            }
            
            // Show debug info
            let pos = self.core_renderer.get_drone_position();
            let vel = self.core_renderer.get_drone_velocity();
            
            ctx.set_fill_style(&"white".into());
            ctx.set_font("12px monospace");
            let info = format!("Pos: ({:.1},{:.1},{:.1}) Vel: ({:.1},{:.1},{:.1})", 
                pos[0], pos[1], pos[2], vel[0], vel[1], vel[2]);
            ctx.fill_text(&info, 10.0, 20.0).ok();
        }
    }
    
    #[wasm_bindgen]
    pub fn reset(&mut self) {
        console_log!("🔄 Reset");
        self.core_renderer.update(0.016, &[InputEvent::Reset]);
    }
    
    #[wasm_bindgen]
    pub fn stop(&mut self) {
        console_log!("🛑 Stop");
        self.core_renderer.update(0.016, &[InputEvent::GentleStop]);
    }
}