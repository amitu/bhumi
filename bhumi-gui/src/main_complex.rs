// Fresh bhumi-gui with proper GPU acceleration
use bhumi::{PhysicsWorld, Camera, InputEvent};
use gilrs::{Gilrs, Button, Axis};
use glam::{Vec3, Mat4};
use log::info;
use std::collections::HashSet;
use std::time::Instant;
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId, Fullscreen},
};

const RENDER_WIDTH: u32 = 320;
const RENDER_HEIGHT: u32 = 240;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    view_proj_matrix: [[f32; 4]; 4],
    time: f32,
    _padding: [f32; 3],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
        0 => Float32x3,  // position
        1 => Float32x3,  // color
    ];
    
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

struct BhumiGpuApp {
    // Core 3D engine
    physics: PhysicsWorld,
    camera: Camera,
    
    // GPU rendering
    device: Option<wgpu::Device>,
    queue: Option<wgpu::Queue>,
    config: Option<wgpu::SurfaceConfiguration>,
    surface: Option<wgpu::Surface<'static>>,
    render_pipeline: Option<wgpu::RenderPipeline>,
    vertex_buffer: Option<wgpu::Buffer>,
    uniform_buffer: Option<wgpu::Buffer>,
    uniform_bind_group: Option<wgpu::BindGroup>,
    
    // Window and input
    window: Option<Window>,
    keys_pressed: HashSet<KeyCode>,
    gamepad: Gilrs,
    
    // Timing and state
    last_frame: Instant,
    frame_count: u64,
    is_fullscreen: bool,
    
    // Physics state
    thrust_force: Vec3,
    rotation_delta: Vec3,
    stopping_mode: StoppingMode,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum StoppingMode {
    None,
    Gentle,
    Emergency,
}

impl BhumiGpuApp {
    fn new() -> Self {
        Self {
            // Core engine
            physics: PhysicsWorld::new(),
            camera: Camera::new(),
            
            // GPU (initialized later)
            device: None,
            queue: None,
            config: None,
            surface: None,
            render_pipeline: None,
            vertex_buffer: None,
            uniform_buffer: None,
            uniform_bind_group: None,
            
            // Window and input
            window: None,
            keys_pressed: HashSet::new(),
            gamepad: Gilrs::new().unwrap_or_else(|_| {
                info!("No gamepad support available");
                Gilrs::new().unwrap()
            }),
            
            // State
            last_frame: Instant::now(),
            frame_count: 0,
            is_fullscreen: false,
            thrust_force: Vec3::ZERO,
            rotation_delta: Vec3::ZERO,
            stopping_mode: StoppingMode::None,
        }
    }
    
    async fn init_gpu(&mut self, window: &Window) {
        let size = window.inner_size();
        
        // Create wgpu instance
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        
        // Create surface
        let surface = instance.create_surface(window).unwrap();
        
        // Request adapter
        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }).await.unwrap();
        
        // Get device and queue
        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
            },
            None,
        ).await.unwrap();
        
        // Configure surface
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats.iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
            
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        
        surface.configure(&device, &config);
        
        // Create shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("wireframe_shader"),
            source: wgpu::ShaderSource::Wgsl(WIREFRAME_SHADER.into()),
        });
        
        // Create uniform buffer
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("uniform_buffer"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        // Create bind group layout
        let uniform_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("uniform_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
        });
        
        // Create bind group
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("uniform_bind_group"),
            layout: &uniform_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                }
            ],
        });
        
        // Create render pipeline
        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("render_pipeline_layout"),
            bind_group_layouts: &[&uniform_bind_group_layout],
            push_constant_ranges: &[],
        });
        
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("render_pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        
        // Generate wireframe cube vertices
        let vertices = self.generate_cube_wireframe_vertices();
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vertex_buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        
        info!("GPU initialized: {}×{} @ {:?}", size.width, size.height, surface_format);
        
        // Store everything
        self.device = Some(device);
        self.queue = Some(queue);
        self.config = Some(config);
        self.surface = Some(surface);
        self.render_pipeline = Some(render_pipeline);
        self.vertex_buffer = Some(vertex_buffer);
        self.uniform_buffer = Some(uniform_buffer);
        self.uniform_bind_group = Some(uniform_bind_group);
    }
    
    fn generate_cube_wireframe_vertices(&self) -> Vec<Vertex> {
        // Simple test cube wireframe
        let white = [1.0, 1.0, 1.0];
        let red = [1.0, 0.0, 0.0];
        
        vec![
            // Test triangle
            Vertex { position: [-0.5, -0.5, 0.0], color: red },
            Vertex { position: [0.5, -0.5, 0.0], color: white },
            Vertex { position: [0.0, 0.5, 0.0], color: white },
            // More vertices will be generated procedurally later
        ]
    }
    
    fn handle_input(&mut self) {
        // Handle gamepad input
        while let Some(gilrs::Event { id: _, event, time: _ }) = self.gamepad.next_event() {
            info!("Gamepad event: {:?}", event);
        }
        
        // Convert input to physics forces
        self.thrust_force = Vec3::ZERO;
        self.rotation_delta = Vec3::ZERO;
        
        // Keyboard input (same as terminal)
        for key in &self.keys_pressed {
            match key {
                KeyCode::KeyW => self.thrust_force.z += 0.3,
                KeyCode::KeyS => self.thrust_force.z -= 0.3,
                KeyCode::KeyA => self.thrust_force.x -= 0.3,
                KeyCode::KeyD => self.thrust_force.x += 0.3,
                KeyCode::Space => self.thrust_force.y += 0.5,
                KeyCode::KeyC => self.thrust_force.y -= 0.5,
                
                KeyCode::KeyJ => self.rotation_delta.y -= 0.02,
                KeyCode::KeyL => self.rotation_delta.y += 0.02,
                KeyCode::KeyI => self.rotation_delta.x -= 0.02,
                KeyCode::KeyK => self.rotation_delta.x += 0.02,
                _ => {}
            }
        }
        
        // TODO: Apply gamepad analog sticks
    }
    
    fn update_physics(&mut self, dt: f32) {
        // Apply stopping modes
        match self.stopping_mode {
            StoppingMode::Gentle => self.physics.gentle_stop(),
            StoppingMode::Emergency => self.physics.emergency_brake(),
            StoppingMode::None => {},
        }
        
        // Apply rotation delta if any
        if self.rotation_delta.length() > 0.001 {
            let rotation_delta_rapier = rapier3d::prelude::Vector::new(
                self.rotation_delta.x, self.rotation_delta.y, self.rotation_delta.z
            );
            self.physics.apply_rotation_delta(rotation_delta_rapier);
        }
        
        // Step physics
        let thrust_rapier = rapier3d::prelude::Vector::new(
            self.thrust_force.x, self.thrust_force.y, self.thrust_force.z
        );
        let drone_pos = self.physics.step(dt, thrust_rapier);
        let drone_rot = self.physics.get_drone_rotation();
        
        // Update camera
        self.camera.update(drone_pos, drone_rot);
    }
    
    fn render(&mut self) {
        let Some(ref device) = self.device else { return };
        let Some(ref queue) = self.queue else { return };
        let Some(ref surface) = self.surface else { return };
        let Some(ref render_pipeline) = self.render_pipeline else { return };
        let Some(ref vertex_buffer) = self.vertex_buffer else { return };
        let Some(ref uniform_buffer) = self.uniform_buffer else { return };
        let Some(ref uniform_bind_group) = self.uniform_bind_group else { return };
        
        // Update uniforms
        let view_proj = self.camera.get_view_projection_matrix();
        let uniforms = Uniforms {
            view_proj_matrix: [
                [view_proj[(0, 0)], view_proj[(0, 1)], view_proj[(0, 2)], view_proj[(0, 3)]],
                [view_proj[(1, 0)], view_proj[(1, 1)], view_proj[(1, 2)], view_proj[(1, 3)]],
                [view_proj[(2, 0)], view_proj[(2, 1)], view_proj[(2, 2)], view_proj[(2, 3)]],
                [view_proj[(3, 0)], view_proj[(3, 1)], view_proj[(3, 2)], view_proj[(3, 3)]],
            ],
            time: self.frame_count as f32 * 0.016,
            _padding: [0.0; 3],
        };
        
        queue.write_buffer(uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
        
        // Get surface texture
        let output = surface.get_current_texture().unwrap();
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        // Create command encoder
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("render_encoder"),
        });
        
        // Render pass
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 20.0 / 255.0,
                            g: 20.0 / 255.0, 
                            b: 30.0 / 255.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            
            render_pass.set_pipeline(render_pipeline);
            render_pass.set_bind_group(0, uniform_bind_group, &[]);
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            
            // Draw wireframe (for now just test triangle)
            render_pass.draw(0..3, 0..1);
        }
        
        queue.submit(std::iter::once(encoder.finish()));
        output.present();
        
        self.frame_count += 1;
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

impl ApplicationHandler for BhumiGpuApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Create window with adaptive scaling
        let monitor = event_loop.primary_monitor().unwrap();
        let monitor_size = monitor.size();
        
        // Calculate good window size (at least 2x scale, up to 80% of monitor)
        let scale_x = (monitor_size.width * 8 / 10) / RENDER_WIDTH;
        let scale_y = (monitor_size.height * 8 / 10) / RENDER_HEIGHT;
        let scale = std::cmp::min(scale_x, scale_y).max(2);
        
        let window_size = PhysicalSize::new(RENDER_WIDTH * scale, RENDER_HEIGHT * scale);
        
        let window = event_loop.create_window(
            Window::default_attributes()
                .with_title("Bhumi 3D - GPU Accelerated")
                .with_inner_size(window_size)
        ).unwrap();
        
        info!("Window created: {}×{} ({}x scale)", window_size.width, window_size.height, scale);
        
        // Initialize GPU asynchronously
        pollster::block_on(self.init_gpu(&window));
        
        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            
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
                        match key_code {
                            KeyCode::Escape | KeyCode::KeyQ => event_loop.exit(),
                            KeyCode::F11 => self.toggle_fullscreen(),
                            KeyCode::Digit0 => {
                                // Reset
                                self.physics.reset_drone();
                                self.stopping_mode = StoppingMode::None;
                            },
                            KeyCode::Digit9 => {
                                self.stopping_mode = StoppingMode::Gentle;
                            },
                            _ => {
                                self.keys_pressed.insert(key_code);
                                if matches!(key_code, KeyCode::KeyW | KeyCode::KeyA | KeyCode::KeyS | KeyCode::KeyD | KeyCode::Space | KeyCode::KeyC) {
                                    self.stopping_mode = StoppingMode::None; // Cancel stopping with thrust
                                }
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
                
                // Handle input and update physics
                self.handle_input();
                self.update_physics(dt);
                
                // Render with GPU
                self.render();
                
                // Request next frame
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            
            WindowEvent::Resized(new_size) => {
                if let (Some(surface), Some(device), Some(config)) = 
                    (&self.surface, &self.device, &mut self.config) {
                    config.width = new_size.width;
                    config.height = new_size.height;
                    surface.configure(device, config);
                }
            }
            
            _ => {}
        }
    }
}

const WIREFRAME_SHADER: &str = r#"
struct Uniforms {
    view_proj_matrix: mat4x4<f32>,
    time: f32,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
}

@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = uniforms.view_proj_matrix * vec4<f32>(vertex.position, 1.0);
    out.color = vertex.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
"#;

fn main() {
    env_logger::init();
    
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    
    let mut app = BhumiGpuApp::new();
    
    info!("🚀 Starting Bhumi GUI with true GPU 3D rendering");
    info!("🎮 Controls: WASD=fly, IJKL=rotate, 0=reset, 9=stop, F11=fullscreen");
    
    event_loop.run_app(&mut app).unwrap();
}