// #[cfg(target_arch = "wasm32")]
// use wasm_bindgen::prelude::*;

// This will store the state of our game
pub struct State {
    window: std::sync::Arc<winit::window::Window>,
}

impl State {
    // We don't need this to be async right now,
    // but we will in the next tutorial
    pub async fn new(window: std::sync::Arc<winit::window::Window>) -> anyhow::Result<Self> {
        Ok(Self { window })
    }

    pub fn resize(&mut self, _width: u32, _height: u32) {
        // We'll do stuff here in the next tutorial
    }

    pub fn render(&mut self) {
        self.window.request_redraw();

        // We'll do more stuff here in the next tutorial
    }
}

pub struct App {
    #[cfg(target_arch = "wasm32")]
    proxy: Option<winit::event_loop::EventLoopProxy<State>>,
    state: Option<State>,
    #[cfg(not(target_arch = "wasm32"))]
    sdl_context: sdl2::Sdl,
    #[cfg(not(target_arch = "wasm32"))]
    game_controller_subsystem: sdl2::GameControllerSubsystem,
    #[cfg(not(target_arch = "wasm32"))]
    controllers: std::collections::HashMap<u32, sdl2::controller::GameController>,
}

impl App {
    #[allow(clippy::new_without_default)]
    pub fn new(
        #[cfg(target_arch = "wasm32")] event_loop: &winit::event_loop::EventLoop<State>,
    ) -> Self {
        #[cfg(target_arch = "wasm32")]
        let proxy = Some(event_loop.create_proxy());

        #[cfg(not(target_arch = "wasm32"))]
        let sdl_context = sdl2::init().unwrap();
        #[cfg(not(target_arch = "wasm32"))]
        let _joystick_subsystem = sdl_context.joystick().unwrap();
        #[cfg(not(target_arch = "wasm32"))]
        let game_controller_subsystem = sdl_context.game_controller().unwrap();

        // Print available controllers
        #[cfg(not(target_arch = "wasm32"))]
        {
            let num_joysticks = game_controller_subsystem.num_joysticks().unwrap_or(0);
            println!("Found {} joystick(s)", num_joysticks);
            for i in 0..num_joysticks {
                if game_controller_subsystem.is_game_controller(i) {
                    println!(
                        "  Controller {}: {}",
                        i,
                        game_controller_subsystem
                            .name_for_index(i)
                            .unwrap_or_else(|_| "Unknown".to_string())
                    );
                }
            }
        }

        Self {
            state: None,
            #[cfg(target_arch = "wasm32")]
            proxy,
            #[cfg(not(target_arch = "wasm32"))]
            sdl_context,
            #[cfg(not(target_arch = "wasm32"))]
            game_controller_subsystem,
            #[cfg(not(target_arch = "wasm32"))]
            controllers: std::collections::HashMap::new(),
        }
    }
}

impl winit::application::ApplicationHandler<State> for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        #[allow(unused_mut)]
        let mut window_attributes = winit::window::Window::default_attributes();

        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowAttributesExtWebSys;

            const CANVAS_ID: &str = "canvas";

            let window = wgpu::web_sys::window().unwrap_throw();
            let document = window.document().unwrap_throw();
            let canvas = document.get_element_by_id(CANVAS_ID).unwrap_throw();
            let html_canvas_element = canvas.unchecked_into();
            window_attributes = window_attributes.with_canvas(Some(html_canvas_element));
        }

        let window = std::sync::Arc::new(event_loop.create_window(window_attributes).unwrap());

        #[cfg(not(target_arch = "wasm32"))]
        {
            // If we are not on web we can use pollster to
            // await the
            self.state = Some(pollster::block_on(State::new(window)).unwrap());
        }

        #[cfg(target_arch = "wasm32")]
        {
            // Run the future asynchronously and use the
            // proxy to send the results to the event loop
            if let Some(proxy) = self.proxy.take() {
                wasm_bindgen_futures::spawn_local(async move {
                    assert!(
                        proxy
                            .send_event(
                                State::new(window)
                                    .await
                                    .expect("Unable to create canvas!!!")
                            )
                            .is_ok()
                    )
                });
            }
        }
    }

    #[allow(unused_mut)]
    fn user_event(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop, mut event: State) {
        #[cfg(target_arch = "wasm32")]
        {
            event.window.request_redraw();
            event.resize(
                event.window.inner_size().width,
                event.window.inner_size().height,
            );
        }
        self.state = Some(event);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let state = match &mut self.state {
            Some(canvas) => canvas,
            None => return,
        };

        match event {
            winit::event::WindowEvent::CloseRequested => event_loop.exit(),
            winit::event::WindowEvent::Resized(size) => state.resize(size.width, size.height),
            winit::event::WindowEvent::RedrawRequested => {
                state.render();
            }
            winit::event::WindowEvent::KeyboardInput {
                event:
                    winit::event::KeyEvent {
                        physical_key: winit::keyboard::PhysicalKey::Code(code),
                        state,
                        ..
                    },
                ..
            } => {
                if let (winit::keyboard::KeyCode::Escape, true) = (code, state.is_pressed()) {
                    event_loop.exit()
                }
            }
            _ => {}
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        let mut event_pump = self.sdl_context.event_pump().unwrap();
        for event in event_pump.poll_iter() {
            match event {
                sdl2::event::Event::ControllerDeviceAdded { which, .. } => {
                    println!("Controller {} connected", which);
                    if let Ok(controller) = self.game_controller_subsystem.open(which) {
                        println!("  Opened: {}", controller.name());
                        self.controllers.insert(which, controller);
                    }
                }
                sdl2::event::Event::ControllerDeviceRemoved { which, .. } => {
                    println!("Controller {} disconnected", which);
                    self.controllers.remove(&which);
                }
                sdl2::event::Event::ControllerButtonDown { which, button, .. } => {
                    println!("Controller {} button {:?} pressed", which, button);
                }
                sdl2::event::Event::ControllerButtonUp { which, button, .. } => {
                    println!("Controller {} button {:?} released", which, button);
                }
                sdl2::event::Event::ControllerAxisMotion {
                    which, axis, value, ..
                } => {
                    // Only print significant axis movements (deadzone)
                    // Use i32 to avoid overflow when value is i16::MIN (-32768)
                    if (value as i32).abs() > 8000 {
                        println!("Controller {} axis {:?}: {}", which, axis, value);
                    }
                }
                _ => {}
            }
        }
    }
}

pub fn run() -> anyhow::Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
    }
    #[cfg(target_arch = "wasm32")]
    {
        console_log::init_with_level(log::Level::Info).unwrap_throw();
    }

    let event_loop = winit::event_loop::EventLoop::with_user_event().build()?;
    let mut app = App::new(
        #[cfg(target_arch = "wasm32")]
        &event_loop,
    );
    event_loop.run_app(&mut app)?;

    Ok(())
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn run_web() -> Result<(), wasm_bindgen::JsValue> {
    console_error_panic_hook::set_once();
    run().unwrap_throw();

    Ok(())
}
