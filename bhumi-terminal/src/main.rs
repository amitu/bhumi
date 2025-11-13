use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    execute,
    terminal::{self, ClearType},
};
use std::env;
use std::io::{Result, Write, stdout};
use std::time::Duration;

use rapier3d::prelude::*;

const GRID_W: usize = 80;
const GRID_H: usize = 30;

/// Minimal wrapper around Rapier that provides:
/// - a static room (floor + 4 walls + ceiling)
/// - a single dynamic drone body
/// - `step(dt, extra_force)` advances sim and returns drone world translation (x,y,z)
struct PhysicsWorld {
    gravity: Vector<f32>,
    integration_parameters: IntegrationParameters,
    pipeline: PhysicsPipeline,
    island_manager: IslandManager,
    broad_phase: BroadPhaseBvh,
    narrow_phase: NarrowPhase,
    bodies: RigidBodySet,
    colliders: ColliderSet,
    impulse_joints: ImpulseJointSet,
    multibody_joints: MultibodyJointSet,
    ccd_solver: CCDSolver,
    // handles
    drone_handle: RigidBodyHandle,
}

impl PhysicsWorld {
    fn new() -> Self {
        let gravity = Vector::new(0.0, -0.05, 0.0); // extremely low gravity for 10x slower movement
        let mut bodies = RigidBodySet::new();
        let mut colliders = ColliderSet::new();

        // Create drone (dynamic sphere)
        let mut rb = RigidBodyBuilder::dynamic()
            .translation(vector![0.0, 1.0, 0.0])
            .linvel(vector![0.0, 0.0, 0.0])
            .build();
        // set some damping so it doesn't accelerate forever
        rb.set_linear_damping(0.6);
        rb.set_angular_damping(0.8);
        let drone_handle = bodies.insert(rb);
        let drone_collider = ColliderBuilder::ball(0.35)
            .restitution(0.3)
            .friction(0.7)
            .build();
        colliders.insert_with_parent(drone_collider, drone_handle, &mut bodies);

        // Build room: floor + ceiling + 4 walls (simple cuboids)
        let room_half_x = 5.0;
        let room_half_z = 5.0;
        let room_height = 6.0;
        // floor
        colliders.insert(ColliderBuilder::cuboid(room_half_x, 0.1, room_half_z)
            .translation(vector![0.0, -0.1, 0.0]).build());
        // ceiling
        colliders.insert(ColliderBuilder::cuboid(room_half_x, 0.1, room_half_z)
            .translation(vector![0.0, room_height + 0.1, 0.0]).build());
        // walls
        colliders.insert(ColliderBuilder::cuboid(0.1, room_height / 2.0, room_half_z)
            .translation(vector![-room_half_x - 0.1, room_height/2.0 - 0.1, 0.0]).build());
        colliders.insert(ColliderBuilder::cuboid(0.1, room_height / 2.0, room_half_z)
            .translation(vector![room_half_x + 0.1, room_height/2.0 - 0.1, 0.0]).build());
        colliders.insert(ColliderBuilder::cuboid(room_half_x, room_height / 2.0, 0.1)
            .translation(vector![0.0, room_height/2.0 - 0.1, -room_half_z - 0.1]).build());
        colliders.insert(ColliderBuilder::cuboid(room_half_x, room_height / 2.0, 0.1)
            .translation(vector![0.0, room_height/2.0 - 0.1, room_half_z + 0.1]).build());

        Self {
            gravity,
            integration_parameters: IntegrationParameters { dt: 1.0 / 60.0, ..Default::default() },
            pipeline: PhysicsPipeline::new(),
            island_manager: IslandManager::new(),
            broad_phase: BroadPhaseBvh::new(),
            narrow_phase: NarrowPhase::new(),
            bodies,
            colliders,
            impulse_joints: ImpulseJointSet::new(),
            multibody_joints: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(),
            drone_handle,
        }
    }

    /// Step the physics world by `dt` seconds.
    /// `force_world` is a Vector3<f32> in world coords applied to the drone this step (e.g. thrust).
    /// Returns the drone position as [x,y,z].
    fn step(&mut self, dt: f32, force_world: Vector<f32>) -> [f32; 3] {
        // set the integration dt to the provided dt
        self.integration_parameters.dt = dt.max(1.0 / 240.0); // clamp small dt
        // apply force to drone
        if let Some(rb) = self.bodies.get_mut(self.drone_handle) {
            rb.add_force(force_world, true);
        }

        // step the physics pipeline
        self.pipeline.step(
            &self.gravity,
            &self.integration_parameters,
            &mut self.island_manager,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.bodies,
            &mut self.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            &mut self.ccd_solver,
            &(),
            &(),
        );

        // read drone position
        if let Some(rb) = self.bodies.get(self.drone_handle) {
            let t = rb.translation();
            [t.x, t.y, t.z]
        } else {
            [0.0, 0.0, 0.0]
        }
    }
}

fn world_to_grid(x: f32, y: f32, _z: f32) -> Option<(usize, usize)> {
    // Side view: X horizontal, Y vertical (very zoomed in for tiny movements)
    // Very small view area: X [-0.5..0.5], Y [0.8..1.3] (around starting position)
    let minx = -0.5; let maxx = 0.5;
    let miny = 0.8; let maxy = 1.3;
    
    if x < minx || x > maxx || y < miny || y > maxy { return None; }
    
    let fx = (x - minx) / (maxx - minx); // 0..1
    let fy = (y - miny) / (maxy - miny); // 0..1
    
    // Use the inner area (avoid borders) for more space
    let inner_width = GRID_W - 4; // avoid 2-dot border on each side
    let inner_height = GRID_H - 4; // avoid 2-dot border top/bottom
    
    let cx = 2 + (fx * (inner_width - 1) as f32).round() as usize; // offset by 2 for border
    let cy = 2 + ((1.0 - fy) * (inner_height - 1) as f32).round() as usize; // flip Y so up is up
    
    Some((cx, cy))
}

fn generate_splash_grid() -> Vec<String> {
    let mut rows: Vec<String> = Vec::with_capacity(GRID_H);
    let center_word = format!("bhumi v{}", env!("CARGO_PKG_VERSION"));
    let word_len = center_word.chars().count();
    let word_row = GRID_H / 2;
    let word_col_start = (GRID_W.saturating_sub(word_len)) / 2;

    for r in 0..GRID_H {
        // start with spaces (clear center)
        let mut line = vec![' '; GRID_W];
        
        // add dots on borders only
        if r == 0 || r == GRID_H - 1 {
            // top and bottom borders
            for c in 0..GRID_W {
                line[c] = '.';
            }
        } else {
            // left and right borders
            line[0] = '.';
            line[GRID_W - 1] = '.';
        }

        // place splash text in center
        if r == word_row {
            for (i, ch) in center_word.chars().enumerate() {
                if word_col_start + i < GRID_W {
                    line[word_col_start + i] = ch;
                }
            }
        }
        rows.push(line.into_iter().collect());
    }
    rows
}

fn generate_physics_grid() -> Vec<String> {
    let mut rows: Vec<String> = Vec::with_capacity(GRID_H);

    for r in 0..GRID_H {
        // start with spaces (clear center)
        let mut line = vec![' '; GRID_W];
        
        // add 2-dot wide borders for raw mode display
        if r == 0 || r == 1 || r == GRID_H - 1 || r == GRID_H - 2 {
            // top and bottom borders (2 rows each)
            for c in 0..GRID_W {
                line[c] = '.';
            }
        } else {
            // left and right borders (2 columns each)
            line[0] = '.';
            line[1] = '.';
            line[GRID_W - 1] = '.';
            line[GRID_W - 2] = '.';
        }

        // Add room visualization (side view: X horizontal, Y vertical)
        // Only draw room content in the inner area, avoid overwriting border dots
        if r >= 2 && r < GRID_H - 2 { // skip top/bottom border rows
            for c in 2..(GRID_W-2) { // skip left/right border columns
                let world_x = -5.0 + ((c - 2) as f32 / (GRID_W - 5) as f32) * 10.0; // map inner area to [-5, 5]
                let world_y = 6.0 - ((r - 2) as f32 / (GRID_H - 5) as f32) * 6.0;   // map inner area to [6, 0]
                
                // Draw floor
                if world_y <= 0.3 {
                    line[c] = '#';
                }
                // Draw ceiling  
                else if world_y >= 5.7 {
                    line[c] = '#';
                }
                // Draw left wall
                else if world_x <= -4.5 {
                    line[c] = '|';
                }
                // Draw right wall
                else if world_x >= 4.5 {
                    line[c] = '|';
                }
            }
        }

        rows.push(line.into_iter().collect());
    }
    rows
}

fn draw_grid_at(top: u16, left: u16, stdout: &mut std::io::Stdout) -> Result<()> {
    let rows = generate_splash_grid();
    draw_grid_with_rows(top, left, stdout, rows)
}

fn draw_grid_with_rows(top: u16, left: u16, stdout: &mut std::io::Stdout, rows: Vec<String>) -> Result<()> {
    // Draw rows to terminal at (top,left)
    for (i, row) in rows.into_iter().enumerate() {
        let y = top.saturating_add(i as u16);
        execute!(stdout, cursor::MoveTo(left, y))?;
        write!(stdout, "{}", row)?;
    }
    stdout.flush()?;
    Ok(())
}

fn draw_physics_with_background(term_w: u16, term_h: u16, top: u16, left: u16, stdout: &mut std::io::Stdout, rows: Vec<String>) -> Result<()> {
    // Fill entire screen with dots first
    for y in 0..term_h {
        execute!(stdout, cursor::MoveTo(0, y))?;
        for x in 0..term_w {
            // Check if this position is inside the physics grid area
            let in_physics_area = y >= top && y < top + GRID_H as u16 && x >= left && x < left + GRID_W as u16;
            if in_physics_area {
                // Get the character from the physics grid
                let grid_y = y - top;
                let grid_x = x - left;
                if let Some(row) = rows.get(grid_y as usize) {
                    if let Some(ch) = row.chars().nth(grid_x as usize) {
                        write!(stdout, "{}", ch)?;
                    } else {
                        write!(stdout, " ")?;
                    }
                } else {
                    write!(stdout, " ")?;
                }
            } else {
                write!(stdout, ".")?;
            }
        }
    }
    stdout.flush()?;
    Ok(())
}

fn draw_error_message(term_w: u16, term_h: u16, stdout: &mut std::io::Stdout) -> Result<()> {
    let messages = [
        format!("bhumi v{}", env!("CARGO_PKG_VERSION")),
        String::new(),
        format!("Terminal size: {}×{}", term_w, term_h),
        format!("Minimum required: {}×{}", GRID_W, GRID_H),
        String::new(),
        "Please resize your terminal".to_string(),
    ];

    // Center messages vertically with 2-line padding
    let start_y = if term_h > messages.len() as u16 + 4 {
        (term_h - messages.len() as u16) / 2
    } else {
        2
    };

    for (i, message) in messages.iter().enumerate() {
        let y = start_y + i as u16;
        // Center message horizontally
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

fn print_raw_grid() -> Result<()> {
    let mut phys = PhysicsWorld::new();
    
    // Run a few physics steps to see movement  
    let mut sim_time = 0.0; // SI units: seconds
    for frame in 0..5 {
        let thrust_force = Vector::new(0.01, 0.08, 0.0); // extremely light forces
        let dt = 0.2; // 0.2 second time step for good balance
        let drone_pos = phys.step(dt, thrust_force);
        sim_time += dt;
        
        // Generate grid with drone
        let mut rows = generate_physics_grid();
        if let Some((gx, gy)) = world_to_grid(drone_pos[0], drone_pos[1], drone_pos[2]) {
            if gy < rows.len() && gx < GRID_W {
                let mut chars: Vec<char> = rows[gy].chars().collect();
                chars[gx] = 'X';
                rows[gy] = chars.into_iter().collect();
            }
        }
        
        println!("=== t={:.1}s - Drone position: x={:.3}m, y={:.3}m, z={:.3}m ===", sim_time, drone_pos[0], drone_pos[1], drone_pos[2]);
        for row in rows {
            println!("{}", row);
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

    let mut stdout = stdout();

    // Setup terminal
    terminal::enable_raw_mode()?;
    execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide)?;

    // Setup physics
    let mut phys = PhysicsWorld::new();
    let mut last_instant = std::time::Instant::now();
    let mut show_physics = false; // start with splash, then auto-switch to physics
    let start_time = std::time::Instant::now();

    // Initial draw
    let (mut term_w, mut term_h) = terminal::size()?;
    // main loop: redraw on resize or on a small timeout; quit on 'q' or Esc.
    loop {
        // clear area (clear entire screen to keep simple)
        execute!(
            stdout,
            terminal::Clear(ClearType::All),
            cursor::MoveTo(0, 0)
        )?;

        // Auto-switch from splash to physics after 0.5 seconds
        if !show_physics && start_time.elapsed().as_millis() >= 500 {
            show_physics = true;
        }

        // Check if terminal is too small
        let is_too_small = term_w < GRID_W as u16 || term_h < GRID_H as u16;

        if is_too_small {
            draw_error_message(term_w, term_h, &mut stdout)?;
        } else if show_physics {
            // compute dt
            let now = std::time::Instant::now();
            let dt = (now - last_instant).as_secs_f32();
            last_instant = now;

            // decide force this step (extremely light thrust for 10x slower movement)
            let thrust_force = Vector::new(0.01, 0.08, 0.0); // extremely light forces
            let drone_pos = phys.step(dt, thrust_force); // returns [x,y,z]

            // generate grid and place drone
            let mut rows = generate_physics_grid();
            if let Some((gx, gy)) = world_to_grid(drone_pos[0], drone_pos[1], drone_pos[2]) {
                if gy < rows.len() && gx < GRID_W {
                    let mut chars: Vec<char> = rows[gy].chars().collect();
                    chars[gx] = 'X';
                    rows[gy] = chars.into_iter().collect();
                }
            }

            // compute top-left to center the GRID inside terminal with 2-line padding
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

            draw_physics_with_background(term_w, term_h, top, left, &mut stdout, rows)?;
        } else {
            // show splash screen
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

            draw_grid_at(top, left, &mut stdout)?;
        }

        // wait for events with a small timeout so we react to resize/keys
        if event::poll(Duration::from_millis(200))? {
            match event::read()? {
                Event::Key(k) => {
                    if k.code == KeyCode::Char('q') || k.code == KeyCode::Esc {
                        break;
                    }
                }
                Event::Resize(w, h) => {
                    term_w = w;
                    term_h = h;
                    // loop will redraw with new size
                }
                _ => {}
            }
        } else {
            // timeout expired -> loop and redraw (keeps center even if terminal changed without Resize event)
            let (w, h) = terminal::size()?;
            term_w = w;
            term_h = h;
        }
    }

    // restore terminal
    execute!(stdout, cursor::Show, terminal::LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;
    Ok(())
}
