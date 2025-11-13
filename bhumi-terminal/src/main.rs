use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    execute,
    terminal::{self, ClearType},
};
use std::env;
use std::io::{Result, Write, stdout};
use std::time::Duration;

use bhumi::{PixelRenderer, PixelBuffer, Renderer, InputEvent, CameraMode, RenderMode};

const GRID_W: usize = 80;
const GRID_H: usize = 30;

/// Terminal renderer implementing the PixelRenderer trait
struct TerminalRenderer {
    should_exit: bool,
    show_physics: bool,
    start_time: std::time::Instant,
    render_mode: RenderMode,
}

impl PixelRenderer for TerminalRenderer {
    fn new() -> Self {
        Self {
            should_exit: false,
            show_physics: false,
            start_time: std::time::Instant::now(),
            render_mode: RenderMode::Braille,
        }
    }

    fn render_frame(&mut self, buffer: &PixelBuffer) -> Result<()> {
        let mut stdout = stdout();

        // Auto-switch from splash to physics after 0.5 seconds
        if !self.show_physics && self.start_time.elapsed().as_millis() >= 500 {
            self.show_physics = true;
        }

        // Get terminal size
        let (term_w, term_h) = terminal::size()?;
        let is_too_small = term_w < GRID_W as u16 || term_h < GRID_H as u16;

        // Clear screen
        execute!(stdout, terminal::Clear(ClearType::All), cursor::MoveTo(0, 0))?;

        if is_too_small {
            self.draw_error_message(term_w, term_h, &mut stdout)?;
        } else if self.show_physics {
            // Convert pixel buffer using current render mode
            match self.render_mode {
                RenderMode::Braille => self.draw_pixel_buffer_as_braille(buffer, term_w, term_h, &mut stdout)?,
                RenderMode::Block => self.draw_pixel_buffer_as_blocks(buffer, term_w, term_h, &mut stdout)?,
                RenderMode::Ascii => self.draw_pixel_buffer_as_ascii(buffer, term_w, term_h, &mut stdout)?,
            }
        } else {
            // Show splash screen
            self.draw_splash_screen(term_w, term_h, &mut stdout)?;
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
                                // Toggle render mode
                                self.render_mode = match self.render_mode {
                                    RenderMode::Braille => RenderMode::Block,
                                    RenderMode::Block => RenderMode::Ascii,
                                    RenderMode::Ascii => RenderMode::Braille,
                                };
                                events.push(InputEvent::ToggleRenderMode);
                            },
                            KeyCode::Char('0') => events.push(InputEvent::Reset),
                            KeyCode::Char('1') => events.push(InputEvent::CameraMode(CameraMode::FirstPerson)),
                            KeyCode::Char('2') => events.push(InputEvent::CameraMode(CameraMode::ThirdPerson)),
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
    /// Convert pixel buffer to ASCII art and render to terminal
    fn draw_pixel_buffer_as_ascii(&self, buffer: &PixelBuffer, term_w: u16, term_h: u16, stdout: &mut std::io::Stdout) -> Result<()> {
        // Calculate position to center the ASCII art
        let left = if term_w as i32 - GRID_W as i32 > 0 {
            ((term_w as usize - GRID_W) / 2) as u16
        } else {
            0u16
        };
        let top = if term_h as i32 - GRID_H as i32 > 4 {
            ((term_h as usize - GRID_H) / 2) as u16
        } else {
            2u16
        };

        // Fill background with dots
        for y in 0..term_h {
            execute!(stdout, cursor::MoveTo(0, y))?;
            for x in 0..term_w {
                let in_ascii_area = y >= top && y < top + GRID_H as u16 && x >= left && x < left + GRID_W as u16;
                if in_ascii_area {
                    // Convert pixel to ASCII character
                    let ascii_x = x - left;
                    let ascii_y = y - top;
                    let char = self.pixel_buffer_to_ascii_char(buffer, ascii_x as usize, ascii_y as usize);
                    write!(stdout, "{}", char)?;
                } else {
                    write!(stdout, ".")?;
                }
            }
        }

        stdout.flush()?;
        Ok(())
    }

    /// Convert a single pixel buffer position to ASCII character
    pub fn pixel_buffer_to_ascii_char(&self, buffer: &PixelBuffer, ascii_x: usize, ascii_y: usize) -> char {
        // Map ASCII position to pixel buffer coordinates
        let pixel_x = (ascii_x as f32 / GRID_W as f32 * buffer.width as f32) as u32;
        let pixel_y = (ascii_y as f32 / GRID_H as f32 * buffer.height as f32) as u32;

        // Sample pixel and convert to brightness
        if let Some(pixel) = buffer.get_pixel(pixel_x, pixel_y) {
            let brightness = (pixel[0] as f32 * 0.299 + pixel[1] as f32 * 0.587 + pixel[2] as f32 * 0.114) / 255.0;
            self.brightness_to_ascii(brightness)
        } else {
            ' '
        }
    }

    /// Convert brightness (0.0-1.0) to ASCII character
    fn brightness_to_ascii(&self, brightness: f32) -> char {
        let chars = " .:-=+*#%@";
        let index = (brightness * (chars.len() - 1) as f32) as usize;
        chars.chars().nth(index).unwrap_or(' ')
    }

    /// Convert pixel buffer to Braille characters (2×4 pixels per char)
    fn draw_pixel_buffer_as_braille(&self, buffer: &PixelBuffer, term_w: u16, term_h: u16, stdout: &mut std::io::Stdout) -> Result<()> {
        let left = if term_w as i32 - GRID_W as i32 > 0 {
            ((term_w as usize - GRID_W) / 2) as u16
        } else { 0u16 };
        let top = if term_h as i32 - GRID_H as i32 > 4 {
            ((term_h as usize - GRID_H) / 2) as u16
        } else { 2u16 };

        // Fill background with dots
        for y in 0..term_h {
            execute!(stdout, cursor::MoveTo(0, y))?;
            for x in 0..term_w {
                let in_braille_area = y >= top && y < top + GRID_H as u16 && x >= left && x < left + GRID_W as u16;
                if in_braille_area {
                    let braille_x = x - left;
                    let braille_y = y - top;
                    let char = self.pixel_buffer_to_braille_char(buffer, braille_x as usize, braille_y as usize);
                    write!(stdout, "{}", char)?;
                } else {
                    write!(stdout, ".")?;
                }
            }
        }
        stdout.flush()?;
        Ok(())
    }

    /// Convert pixel buffer position to Braille character (2×4 pixel sampling)
    fn pixel_buffer_to_braille_char(&self, buffer: &PixelBuffer, braille_x: usize, braille_y: usize) -> char {
        // Each Braille character represents 2×4 pixels
        // Braille pattern:
        // 1 4
        // 2 5  
        // 3 6
        // 7 8
        
        let pixel_start_x = (braille_x as f32 * 2.0 / GRID_W as f32 * buffer.width as f32) as u32;
        let pixel_start_y = (braille_y as f32 * 4.0 / GRID_H as f32 * buffer.height as f32) as u32;
        
        let mut braille_value = 0u8;
        let braille_bits = [0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80]; // Bit positions
        
        // Sample 2×4 pixels
        for dy in 0..4 {
            for dx in 0..2 {
                let px = pixel_start_x + dx;
                let py = pixel_start_y + dy;
                
                if let Some(pixel) = buffer.get_pixel(px, py) {
                    let brightness = (pixel[0] as f32 * 0.299 + pixel[1] as f32 * 0.587 + pixel[2] as f32 * 0.114) / 255.0;
                    if brightness > 0.5 { // Threshold for bright pixel
                        let bit_index = (dy * 2 + dx) as usize;
                        if bit_index < 8 {
                            braille_value |= braille_bits[bit_index];
                        }
                    }
                }
            }
        }
        
        // Convert to Braille Unicode character
        char::from_u32(0x2800 + braille_value as u32).unwrap_or(' ')
    }

    /// Convert pixel buffer to block characters with color
    fn draw_pixel_buffer_as_blocks(&self, buffer: &PixelBuffer, term_w: u16, term_h: u16, stdout: &mut std::io::Stdout) -> Result<()> {
        // TODO: Implement block character rendering
        self.draw_pixel_buffer_as_ascii(buffer, term_w, term_h, stdout)
    }

    /// Draw splash screen with version info
    fn draw_splash_screen(&self, term_w: u16, term_h: u16, stdout: &mut std::io::Stdout) -> Result<()> {
        let splash_text = format!("bhumi v{}", env!("CARGO_PKG_VERSION"));
        
        // Center the splash text
        let left = if term_w as i32 - splash_text.len() as i32 > 0 {
            ((term_w as usize - splash_text.len()) / 2) as u16
        } else {
            0u16
        };
        let top = term_h / 2;

        execute!(stdout, cursor::MoveTo(left, top))?;
        write!(stdout, "{}", splash_text)?;
        stdout.flush()?;
        Ok(())
    }

    /// Draw error message for terminal too small
    fn draw_error_message(&self, term_w: u16, term_h: u16, stdout: &mut std::io::Stdout) -> Result<()> {
        let messages = [
            format!("bhumi v{}", env!("CARGO_PKG_VERSION")),
            String::new(),
            format!("Terminal size: {}×{}", term_w, term_h),
            format!("Minimum required: {}×{}", GRID_W, GRID_H),
            String::new(),
            "Please resize your terminal".to_string(),
        ];

        let start_y = if term_h > messages.len() as u16 + 4 {
            (term_h - messages.len() as u16) / 2
        } else {
            2
        };

        for (i, message) in messages.iter().enumerate() {
            let y = start_y + i as u16;
            let x = if term_w > message.len() as u16 {
                (term_w - message.len() as u16) / 2
            } else {
                0
            };
            execute!(stdout, cursor::MoveTo(x, y))?;
            write!(stdout, "{}", message)?;
        }
        stdout.flush()?;
        Ok(())
    }
}

/// Raw mode for debugging (prints frames to stdout)
fn print_raw_grid() -> Result<()> {
    let mut renderer = Renderer::new();
    let mut sim_time = 0.0;

    // Create a terminal renderer to convert pixels to ASCII
    let terminal_renderer = TerminalRenderer::new();

    for _frame in 0..6 {
        let dt = 0.2;
        
        // No automatic thrust in raw mode - just let initial velocity carry it
        let input_events = vec![];
        renderer.update(dt, &input_events);
        renderer.render();
        sim_time += dt;

        let drone_pos = renderer.get_drone_position();
        println!("=== t={:.1}s - Drone position: x={:.3}m, y={:.3}m, z={:.3}m ===", sim_time, drone_pos[0], drone_pos[1], drone_pos[2]);
        
        // Show actual Braille rendering of the pixel buffer
        let terminal_renderer = TerminalRenderer::new();
        for y in 0..GRID_H {
            for x in 0..GRID_W {
                let char = terminal_renderer.pixel_buffer_to_braille_char(&renderer.buffer, x, y);
                print!("{}", char);
            }
            println!();
        }
        println!();
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

    // Setup terminal
    terminal::enable_raw_mode()?;
    execute!(stdout(), terminal::EnterAlternateScreen, cursor::Hide)?;

    // Create renderer instances
    let mut terminal_renderer = TerminalRenderer::new();
    let mut core_renderer = Renderer::new();
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

        // Update 3D world
        core_renderer.update(dt, &input_events);
        core_renderer.render();

        // Render to terminal
        terminal_renderer.render_frame(&core_renderer.buffer)?;

        // Small delay
        std::thread::sleep(Duration::from_millis(33)); // ~30 FPS
    }

    // Restore terminal
    execute!(stdout(), cursor::Show, terminal::LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;
    Ok(())
}