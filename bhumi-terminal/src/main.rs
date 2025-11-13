use crossterm::{
    event::{self, Event, KeyCode},
    terminal,
    cursor, execute,
};
use std::env;
use std::io::{Result, Write};
use std::time::Duration;
use std::fs::OpenOptions;

use bhumi::{PixelRenderer, PixelBuffer, Renderer, InputEvent};
use image::{RgbaImage, DynamicImage};

/// Terminal renderer using viuer for high-quality image display
struct TerminalRenderer {
    should_exit: bool,
    show_physics: bool,
    start_time: std::time::Instant,
    log_file: std::fs::File,
    frame_count: u32,
    render_mode: ViuerMode,
}

#[derive(Debug, Clone, Copy)]
enum ViuerMode {
    Auto,        // Let viuer auto-detect best protocol  
    Block,       // Force block characters with truecolor
    LowRes,      // Smaller image for different look
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
            render_mode: ViuerMode::Auto,
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
                            KeyCode::Tab => {
                                // Toggle viuer rendering mode
                                self.render_mode = match self.render_mode {
                                    ViuerMode::Auto => ViuerMode::Block,
                                    ViuerMode::Block => ViuerMode::LowRes,
                                    ViuerMode::LowRes => ViuerMode::Auto,
                                };
                                self.log(&format!("Switched to render mode: {:?}", self.render_mode));
                            },
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
        // Get terminal size for centering
        let (term_w, term_h) = if let Ok(size) = terminal::size() {
            size
        } else {
            (80, 30) // fallback
        };

        // Fixed dimensions: 80×30 characters for 320×240 pixel buffer
        let image_w = 80;
        let image_h = 30;
        let center_x = if term_w > image_w { (term_w - image_w) / 2 } else { 0 };
        let center_y = if term_h > image_h { (term_h - image_h) / 2 } else { 0 };

        // Configure viuer based on selected mode
        let config = match self.render_mode {
            ViuerMode::Auto => viuer::Config {
                width: Some(image_w as u32),
                height: Some(image_h as u32),
                x: center_x,
                y: center_y as i16,
                absolute_offset: true,
                ..Default::default()
            },
            ViuerMode::Block => viuer::Config {
                width: Some(image_w as u32),
                height: Some(image_h as u32),
                x: center_x,
                y: center_y as i16,
                absolute_offset: true,
                truecolor: true,
                ..Default::default()
            },
            ViuerMode::LowRes => viuer::Config {
                width: Some(40),  // Half size for dramatically different look
                height: Some(15),
                x: center_x + 20, // Center the smaller image
                y: center_y as i16 + 7,
                absolute_offset: true,
                truecolor: false, // Lower color depth for retro feel
                ..Default::default()
            },
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

/// Interactive visual test mode - shows viuer modes one at a time
fn run_visual_test() -> Result<()> {
    terminal::enable_raw_mode()?;
    
    // Create test images that look different for each protocol
    let mut renderer = Renderer::new();
    
    // Track which modes work for summary
    let mut mode_results: Vec<(String, bool, String)> = Vec::new();
    
    // Convert pixel buffer once
    let rgba_bytes: Vec<u8> = renderer.buffer.pixels.iter()
        .flat_map(|pixel| [pixel[0], pixel[1], pixel[2], pixel[3]])
        .collect();
    let rgba_image = RgbaImage::from_raw(renderer.buffer.width, renderer.buffer.height, rgba_bytes)
        .expect("Failed to create image");
    let dynamic_image = DynamicImage::ImageRgba8(rgba_image);

    // Get terminal size for centering
    let (term_w, term_h) = terminal::size()?;
    let center_x = if term_w > 80 { (term_w - 80) / 2 } else { 0 };
    let center_y = if term_h > 30 { (term_h - 30) / 2 } else { 0 };
    
    // Define ALL viuer protocols with sixel feature enabled
    let test_configs = vec![
        ("🤖 Auto Detection (Let viuer choose)", viuer::Config {
            width: Some(80), height: Some(30), x: center_x, y: center_y as i16, 
            ..Default::default()
        }),
        ("🔥 Kitty Graphics Protocol", viuer::Config {
            width: Some(80), height: Some(30), x: center_x, y: center_y as i16,
            use_kitty: true, use_iterm: false, use_sixel: false, truecolor: true,
            ..Default::default()
        }),
        ("🍎 iTerm2 Graphics Protocol", viuer::Config {
            width: Some(80), height: Some(30), x: center_x, y: center_y as i16,
            use_kitty: false, use_iterm: true, use_sixel: false, truecolor: true,
            ..Default::default()
        }),
        ("📺 Sixel Graphics Protocol", viuer::Config {
            width: Some(80), height: Some(30), x: center_x, y: center_y as i16,
            use_kitty: false, use_iterm: false, use_sixel: true, truecolor: true,
            ..Default::default()
        }),
        ("🧱 Block + Truecolor", viuer::Config {
            width: Some(80), height: Some(30), x: center_x, y: center_y as i16,
            use_kitty: false, use_iterm: false, use_sixel: false, truecolor: true,
            ..Default::default()
        }),
        ("🧱 Block + Transparent", viuer::Config {
            width: Some(80), height: Some(30), x: center_x, y: center_y as i16,
            use_kitty: false, use_iterm: false, use_sixel: false, transparent: true, truecolor: true,
            ..Default::default()
        }),
        ("🎨 Block + Low Color", viuer::Config {
            width: Some(80), height: Some(30), x: center_x, y: center_y as i16,
            use_kitty: false, use_iterm: false, use_sixel: false, truecolor: false,
            ..Default::default()
        }),
        ("🔄 Block + Cursor Restore", viuer::Config {
            width: Some(80), height: Some(30), x: center_x, y: center_y as i16,
            use_kitty: false, use_iterm: false, use_sixel: false, restore_cursor: true, truecolor: true,
            ..Default::default()
        }),
    ];

    let mut current_mode = 0;
    
    loop {
        // Clear screen
        print!("\x1b[2J\x1b[H");
        
        // Clear previous results to avoid accumulation bug
        if mode_results.len() > current_mode {
            mode_results.truncate(current_mode);
        }
        
        let (name, config) = &test_configs[current_mode];
        
        // Pre-check if protocol is actually supported by this terminal
        let term_program = std::env::var("TERM_PROGRAM").unwrap_or_default();
        let term = std::env::var("TERM").unwrap_or_default();
        
        let actually_supported = match name {
            n if n.contains("Kitty") => term.contains("kitty") || term_program.contains("kitty"),
            n if n.contains("iTerm2") => term_program.contains("iTerm"),
            n if n.contains("Sixel") => {
                // Very few terminals actually support Sixel
                term.contains("mlterm") || term.contains("xterm") && !term_program.contains("Apple_Terminal")
            },
            _ => true, // Auto and Block modes should work everywhere
        };
        
        let status = if actually_supported {
            println!("Mode {}/{}: {} | ←→ S=summary Q=quit", current_mode + 1, test_configs.len(), name);
            renderer.render(); // Fresh render for each test
            let rgba_bytes: Vec<u8> = renderer.buffer.pixels.iter()
                .flat_map(|pixel| [pixel[0], pixel[1], pixel[2], pixel[3]])
                .collect();
            if let Some(rgba_image) = RgbaImage::from_raw(renderer.buffer.width, renderer.buffer.height, rgba_bytes) {
                let dynamic_image = DynamicImage::ImageRgba8(rgba_image);
                match viuer::print(&dynamic_image, config) {
                    Ok((w, h)) => {
                        mode_results.push((name.to_string(), true, format!("{}×{}", w, h)));
                    },
                    Err(e) => {
                        mode_results.push((name.to_string(), false, e.to_string()));
                    }
                }
            }
        } else {
            println!("Mode {}/{}: {} SKIPPED (Terminal doesn't support) | ←→ S=summary Q=quit", current_mode + 1, test_configs.len(), name);
            mode_results.push((name.to_string(), false, "Not supported by terminal".to_string()));
        };
        
        // Wait for input
        loop {
            if let Ok(Event::Key(k)) = event::read() {
                match k.code {
                    KeyCode::Right | KeyCode::Char(' ') => {
                        current_mode = (current_mode + 1) % test_configs.len();
                        break;
                    },
                    KeyCode::Left => {
                        current_mode = if current_mode == 0 { 
                            test_configs.len() - 1 
                        } else { 
                            current_mode - 1 
                        };
                        break;
                    },
                    KeyCode::Char('s') | KeyCode::Char('S') => {
                        // Show summary
                        print!("\x1b[2J\x1b[H"); // Clear screen and reset cursor
                        show_summary(&mode_results, &test_configs);
                        event::read().ok();
                        break;
                    },
                    KeyCode::Char('q') | KeyCode::Esc => {
                        // Show final summary before exit
                        print!("\x1b[2J\x1b[H");
                        show_summary(&mode_results, &test_configs);
                        execute!(std::io::stdout(), cursor::MoveTo(0, (test_configs.len() + 5) as u16)).ok();
                        print!("Exiting...");
                        std::io::stdout().flush().ok();
                        execute!(std::io::stdout(), cursor::MoveTo(0, (test_configs.len() + 6) as u16)).ok();
                        terminal::disable_raw_mode()?;
                        return Ok(());
                    },
                    _ => {}
                }
            }
        }
    }
}

/// Show summary of all viuer modes and their support status  
fn show_summary(results: &[(String, bool, String)], configs: &[(&str, viuer::Config)]) {
    // Force cursor to position 0,0 using crossterm
    execute!(std::io::stdout(), cursor::MoveTo(0, 0)).ok();
    
    let term_program = std::env::var("TERM_PROGRAM").unwrap_or("Unknown".to_string());
    let supported_count = results.iter().filter(|(_, supported, _)| *supported).count();
    let total_count = configs.len();
    
    println!("Summary: {}/{} modes work on {}", supported_count, total_count, term_program);
    
    for (i, (name, _)) in configs.iter().enumerate() {
        execute!(std::io::stdout(), cursor::MoveTo(0, (i + 2) as u16)).ok();
        if let Some((_, supported, _info)) = results.get(i) {
            if *supported {
                print!("✅ {}", name);
            } else {
                print!("❌ {}", name);
            }
        } else {
            print!("🔄 {}", name);
        }
    }
    
    execute!(std::io::stdout(), cursor::MoveTo(0, (configs.len() + 3) as u16)).ok();
    print!("Press any key...");
    std::io::stdout().flush().ok();
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

    // Check for special flags
    if args.contains(&"--visual-test".to_string()) {
        return run_visual_test();
    }
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