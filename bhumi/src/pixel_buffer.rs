/// Standard pixel buffer used by all renderers
/// Resolution: 320×240 (4:3 aspect ratio)
/// Format: RGBA8 (32-bit per pixel)
#[derive(Clone)]
pub struct PixelBuffer {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<[u8; 4]>, // RGBA
}

impl PixelBuffer {
    /// Create new pixel buffer with standard 320×240 resolution
    pub fn new() -> Self {
        const WIDTH: u32 = 320;
        const HEIGHT: u32 = 240;
        Self {
            width: WIDTH,
            height: HEIGHT,
            pixels: vec![[0, 0, 0, 255]; (WIDTH * HEIGHT) as usize], // Black with full alpha
        }
    }

    /// Clear buffer to specified color
    pub fn clear(&mut self, color: [u8; 4]) {
        self.pixels.fill(color);
    }

    /// Set pixel at coordinates (x, y) to color
    /// Returns true if pixel was set, false if out of bounds
    pub fn set_pixel(&mut self, x: u32, y: u32, color: [u8; 4]) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }
        let index = (y * self.width + x) as usize;
        if index < self.pixels.len() {
            self.pixels[index] = color;
            true
        } else {
            false
        }
    }

    /// Get pixel color at coordinates (x, y)
    /// Returns None if out of bounds
    pub fn get_pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let index = (y * self.width + x) as usize;
        self.pixels.get(index).copied()
    }

    /// Draw a line from (x0, y0) to (x1, y1)
    pub fn draw_line(&mut self, x0: u32, y0: u32, x1: u32, y1: u32, color: [u8; 4]) {
        // Bresenham's line algorithm
        let dx = (x1 as i32 - x0 as i32).abs();
        let dy = (y1 as i32 - y0 as i32).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx - dy;

        let mut x = x0 as i32;
        let mut y = y0 as i32;

        loop {
            self.set_pixel(x as u32, y as u32, color);

            if x == x1 as i32 && y == y1 as i32 {
                break;
            }

            let e2 = 2 * err;
            if e2 > -dy {
                err -= dy;
                x += sx;
            }
            if e2 < dx {
                err += dx;
                y += sy;
            }
        }
    }

    /// Draw a filled rectangle
    pub fn draw_rect(&mut self, x: u32, y: u32, width: u32, height: u32, color: [u8; 4]) {
        for dy in 0..height {
            for dx in 0..width {
                self.set_pixel(x + dx, y + dy, color);
            }
        }
    }
}
