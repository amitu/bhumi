use crossterm::{
    event::{self, Event, KeyCode},
    terminal,
};
use std::env;
use std::io::{Result, Write};
use std::time::Duration;
use std::fs::OpenOptions;

use bhumi::{PixelRenderer, PixelBuffer, Renderer, InputEvent, CameraMode};
use image::{RgbaImage, DynamicImage};

/// Terminal renderer using viuer for high-quality image display
struct TerminalRenderer {
    should_exit: bool,
    show_physics: bool,
    start_time: std::time::Instant,
    log_file: std::fs::File,
    frame_count: u32,
}

impl PixelRenderer for TerminalRenderer {
    fn new() -> Self {
        let log_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open("bhumi_debug.log")
            .expect("Failed to create log file");
            
        Self {
            should_exit: false,
            show_physics: false,
            start_time: std::time::Instant::now(),
            log_file,
            frame_count: 0,
        }
    }

    fn render_frame(&mut self, buffer: &PixelBuffer) -> Result<()> {
        // Auto-switch from splash to physics after 0.5 seconds
        if !self.show_physics && self.start_time.elapsed().as_millis() >= 500 {
            self.show_physics = true;
        }

        // Clear screen
        print!("\x1b[2J\x1b[H"); // Clear screen and move cursor to top

        if self.show_physics {
            // Use viuer to display our pixel buffer
            self.draw_pixel_buffer_with_viuer(buffer)?;
        } else {
            // Show simple splash
            println!("bhumi v{}", env!("CARGO_PKG_VERSION"));
        }

        Ok(())
    }

    fn handle_input(&mut self) -> Vec<InputEvent> {
        let mut events = Vec::new();

        if event::poll(Duration::from_millis(16)).unwrap_or(false) {
            if let Ok(event) = event::read() {
                match event {
                    Event::Key(k) => {
                        match k.code {
                            KeyCode::Char('q') | KeyCode::Esc => {
                                self.should_exit = true;
                                events.push(InputEvent::Exit);
                            },
                            KeyCode::Char('w') | KeyCode::Up => events.push(InputEvent::ThrustForward),
                            KeyCode::Char('s') | KeyCode::Down => events.push(InputEvent::ThrustBackward), 
                            KeyCode::Char('a') | KeyCode::Left => events.push(InputEvent::ThrustLeft),
                            KeyCode::Char('d') | KeyCode::Right => events.push(InputEvent::ThrustRight),
                            KeyCode::Char(' ') => events.push(InputEvent::ThrustUp),
                            KeyCode::Char('c') => events.push(InputEvent::ThrustDown),
                            KeyCode::Char('0') => events.push(InputEvent::Reset),
                            KeyCode::Char('9') => events.push(InputEvent::Stop),
                            _ => {}
                        }
                    },
                    _ => {}
                }
            }
        }

        events
    }

    fn should_exit(&self) -> bool {
        self.should_exit
    }
}

impl TerminalRenderer {
    /// Log debug message to file
    fn log(&mut self, message: &str) {
        use std::io::Write;
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        writeln!(self.log_file, "[{}] {}", timestamp, message).ok();
        self.log_file.flush().ok();
    }

    /// Use viuer to display pixel buffer as high-quality terminal image
    fn draw_pixel_buffer_with_viuer(&self, buffer: &PixelBuffer) -> Result<()> {
        // Configure viuer for our terminal size
        let config = viuer::Config {
            width: Some(80),     // Terminal width in characters
            height: Some(30),    // Terminal height in characters
            absolute_offset: false,
            ..Default::default()
        };

        // Convert our pixel buffer to format that image crate expects
        let rgba_bytes: Vec<u8> = buffer.pixels.iter()
            .flat_map(|pixel| [pixel[0], pixel[1], pixel[2], pixel[3]])
            .collect();

        // Create RgbaImage from raw bytes
        if let Some(rgba_image) = RgbaImage::from_raw(buffer.width, buffer.height, rgba_bytes) {
            // Convert to DynamicImage
            let dynamic_image = DynamicImage::ImageRgba8(rgba_image);
            
            // Display using viuer
            if let Err(e) = viuer::print(&dynamic_image, &config) {
                eprintln!("Viuer error: {}", e);
                // Fallback: show basic info
                println!("Drone rendering... (viuer failed)");
            }
        } else {
            println!("Failed to create image from pixel buffer");
        }

        Ok(())
    }
}

/// Raw mode for debugging
fn print_raw_grid() -> Result<()> {
    let mut renderer = Renderer::new();
    let mut sim_time = 0.0;

    for frame in 0..6 {
        let dt = 0.5; // Longer time steps for visible changes
        
        // No automatic thrust in raw mode - just let initial velocity carry it
        let input_events = vec![];
        renderer.update(dt, &input_events);
        renderer.render();
        sim_time += dt;

        let drone_pos = renderer.get_drone_position();
        println!("=== Frame {} - t={:.1}s - Drone: x={:.3}m, y={:.3}m, z={:.3}m ===", 
                 frame + 1, sim_time, drone_pos[0], drone_pos[1], drone_pos[2]);
        
        // Use viuer for raw mode too
        let config = viuer::Config {
            width: Some(40),   // Smaller for raw debug output
            height: Some(20),
            ..Default::default()
        };

        let rgba_bytes: Vec<u8> = renderer.buffer.pixels.iter()
            .flat_map(|pixel| [pixel[0], pixel[1], pixel[2], pixel[3]])
            .collect();

        // Create RgbaImage for raw mode
        if let Some(rgba_image) = RgbaImage::from_raw(renderer.buffer.width, renderer.buffer.height, rgba_bytes) {
            let dynamic_image = DynamicImage::ImageRgba8(rgba_image);
            if let Err(_) = viuer::print(&dynamic_image, &config) {
                println!("(viuer not available - showing coordinates only)");
            }
        } else {
            println!("Failed to create debug image");
        }
        
        println!();
    }
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    // Check for --raw flag
    if args.contains(&"--raw".to_string()) {
        return print_raw_grid();
    }

    // Setup terminal for raw mode
    terminal::enable_raw_mode()?;

    // Create renderer instances
    let mut terminal_renderer = TerminalRenderer::new();
    terminal_renderer.log("App started - creating core renderer");
    
    let mut core_renderer = Renderer::new();
    let drone_pos = core_renderer.get_drone_position();
    terminal_renderer.log(&format!("Core renderer created - initial drone pos: x={:.3}, y={:.3}, z={:.3}", 
        drone_pos[0], drone_pos[1], drone_pos[2]));
    
    let mut last_instant = std::time::Instant::now();

    // Main loop
    loop {
        let now = std::time::Instant::now();
        let dt = (now - last_instant).as_secs_f32();
        last_instant = now;

        // Handle input
        let input_events = terminal_renderer.handle_input();
        if terminal_renderer.should_exit() {
            break;
        }

        // Log input events
        if !input_events.is_empty() {
            terminal_renderer.log(&format!("Input events: {:?}", input_events));
        }

        // Update 3D world
        core_renderer.update(dt, &input_events);
        core_renderer.render();

        // Log drone position occasionally
        terminal_renderer.frame_count += 1;
        if terminal_renderer.frame_count % 60 == 0 { // Every ~2 seconds
            let pos = core_renderer.get_drone_position();
            let vel = core_renderer.get_drone_velocity();
            terminal_renderer.log(&format!("Frame {}: Drone pos: x={:.3}, y={:.3}, z={:.3}, vel: x={:.3}, y={:.3}, z={:.3}", 
                terminal_renderer.frame_count, pos[0], pos[1], pos[2], vel[0], vel[1], vel[2]));
        }

        // Render to terminal with viuer
        terminal_renderer.render_frame(&core_renderer.buffer)?;

        // Small delay for ~30 FPS
        std::thread::sleep(Duration::from_millis(33));
    }

    // Restore terminal
    terminal::disable_raw_mode()?;
    Ok(())
}