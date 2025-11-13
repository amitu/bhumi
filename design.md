# Bhumi 3D Engine Architecture

## Overview
Bhumi is a modular 3D graphics engine that renders to multiple backends. The core engine handles 3D math, physics, and rendering to a pixel buffer, while backend-specific crates handle display output.

## Goals
- **Modular design**: Clean separation between 3D engine and display backends
- **Multiple renderers**: Terminal (ASCII), GPU (wgpu), future: web, SDL, etc.
- **Physics simulation**: Real-time physics with rapier3d
- **Camera system**: Multiple camera modes (first-person, third-person, free-cam)
- **Cross-platform**: Works anywhere with minimal dependencies

## Architecture

### Crate Structure
```
bhumi/                    # Core 3D engine library
├── Cargo.toml           
├── src/
│   ├── lib.rs           # Public API + PixelRenderer trait
│   ├── pixel_buffer.rs  # 320×240 RGBA buffer management
│   ├── camera.rs        # Camera transforms & projections  
│   ├── world.rs         # 3D world, objects, physics
│   ├── renderer.rs      # 3D→2D rasterization
│   └── physics.rs       # Physics world wrapper

bhumi-terminal/          # Terminal ASCII/Unicode renderer
├── Cargo.toml
├── src/
│   ├── main.rs          # Terminal application
│   ├── ascii_converter.rs # Pixel buffer → ASCII conversion
│   ├── terminal.rs      # Crossterm integration
│   └── input.rs         # Keyboard input handling

bhumi-wgpu/              # GPU renderer (future)
├── Cargo.toml
├── src/
│   ├── main.rs          # Desktop application  
│   ├── gpu_renderer.rs  # wgpu rendering pipeline
│   └── window.rs        # winit window management
```

## Core Design

### Pixel Buffer
- **Resolution**: 320×240 (4:3 aspect ratio)
- **Format**: RGBA8 (32-bit per pixel)
- **Memory**: ~300KB per frame
- **Usage**: All renderers work with this standard buffer

### Camera System
#### Camera Modes
1. **First-person**: Camera at drone position, world moves relative to drone
2. **Third-person**: Camera follows behind/above drone
3. **Free-cam**: User-controlled camera (debug mode)

#### Projection
- **Type**: Perspective projection
- **FOV**: 60° vertical field of view
- **Near/Far**: 0.1m to 100m clipping planes
- **Viewport**: Maps to 320×240 pixel buffer

### Physics Integration
- **Engine**: rapier3d for physics simulation
- **World**: Static room geometry (walls, floor, ceiling)
- **Drone**: Dynamic rigid body with thrust forces
- **Timestep**: Fixed timestep for deterministic physics

### World Coordinate System
- **Units**: SI units (meters, seconds)  
- **Axes**: Right-handed coordinate system
  - X: Right/left movement
  - Y: Up/down movement  
  - Z: Forward/backward movement
- **Scale**: Room is ~10m × 10m × 6m

## Renderer Interface

### Core Trait
```rust
pub trait PixelRenderer {
    fn new() -> Self;
    fn render_frame(&mut self, buffer: &PixelBuffer) -> Result<()>;
    fn handle_input(&mut self) -> Vec<InputEvent>;
    fn should_exit(&self) -> bool;
}

pub struct PixelBuffer {
    pub width: u32,    // 320
    pub height: u32,   // 240  
    pub pixels: Vec<[u8; 4]>, // RGBA pixels
}

pub enum InputEvent {
    ThrustUp,
    ThrustDown, 
    ThrustLeft,
    ThrustRight,
    ThrustForward,
    ThrustBackward,
    CameraMode(CameraMode),
    Exit,
}
```

## Terminal Renderer Specifics

### ASCII Conversion
- **Dithering**: Floyd-Steinberg dithering for smooth gradients
- **Character set**: ASCII characters by brightness: ` .:-=+*#%@`
- **Color**: ANSI color codes for terminals that support it
- **Fallback**: Pure ASCII for compatibility

### Terminal Integration  
- **Library**: crossterm for cross-platform terminal control
- **Features**: Raw mode, alternate screen, cursor hiding
- **Input**: Real-time keyboard input without blocking
- **Resize**: Dynamic handling of terminal resize events

### Display Layout
```
................................................................
................................................................
..┌────────────────────────────────────────────────────────┐..
..│################  ########  ######################  ####│..
..│####  ██████████  ########  ███              ███  ####│..
..│####  ██      ██  ########  ███      X       ███  ####│..
..│####  ██      ██  ########  ███              ███  ####│..
..│####  ██████████  ########  ######################  ####│..
..└────────────────────────────────────────────────────────┘..
................................................................
Status: X=1.23m Y=2.45m Z=0.67m | Thrust=0.8N | t=12.3s
```

## Implementation Phases

### Phase 1: Core Engine Foundation
- [ ] Create `bhumi` crate with basic structure
- [ ] Implement `PixelBuffer` and basic 3D math
- [ ] Move physics from `bhumi-terminal` to `bhumi` 
- [ ] Add camera system with first-person mode
- [ ] Software rasterization for basic shapes

### Phase 2: Terminal Renderer
- [ ] Refactor `bhumi-terminal` to use `bhumi` core
- [ ] Implement `PixelRenderer` trait for terminal
- [ ] Add ASCII conversion with dithering
- [ ] Real-time input handling for drone control
- [ ] Multiple camera mode support

### Phase 3: Enhanced Rendering
- [ ] Add proper 3D mesh rendering
- [ ] Lighting and shading models
- [ ] Texture mapping support
- [ ] Enhanced ASCII character sets
- [ ] Unicode/ANSI color support

### Phase 4: GPU Renderer (Future)
- [ ] Create `bhumi-wgpu` crate
- [ ] GPU-accelerated rendering pipeline
- [ ] Advanced lighting and effects
- [ ] High-resolution rendering

## Technical Specifications

### Performance Targets
- **Terminal**: 30+ FPS at 320×240 → 80×30 ASCII
- **Memory**: <50MB total memory usage
- **CPU**: Smooth on modern CPUs, acceptable on older hardware
- **Compatibility**: Works on Linux, macOS, Windows terminals

### Dependencies
#### bhumi (core)
- rapier3d: Physics simulation
- nalgebra/glam: 3D mathematics  
- bytemuck: Safe memory casting

#### bhumi-terminal
- crossterm: Terminal control
- bhumi: Core engine

### File Formats (Future)
- **Models**: glTF 2.0 for 3D assets
- **Textures**: PNG/JPEG for texture maps
- **Scenes**: Custom TOML format for scene description

## Development Notes

### Current Status
- Basic physics simulation working
- ASCII terminal output functional  
- Need to extract core engine and implement pixel buffer pipeline

### Known Limitations
- Software rendering only (until GPU backend)
- Fixed resolution (320×240)
- Simple ASCII character set
- No texture mapping yet

### Future Enhancements
- Dynamic resolution scaling
- Extended Unicode character sets
- Web renderer (wasm + canvas)
- VR/AR support potential
- Networked physics simulation