mod texture;
mod model;
mod control;
mod scene;
mod gui;

use winit::{
    window::{Window, WindowBuilder},
    event_loop::{EventLoop, ControlFlow},
    event::{Event, DeviceEvent, WindowEvent, KeyboardInput, ElementState, VirtualKeyCode}
};

#[cfg(target_arch="wasm32")]
use wasm_bindgen::prelude::*;

struct State {
    window: Window,
    size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    scene: scene::Scene,
    indicator_renderer: renderer::indicator::IndicatorRenderer,
    gui_renderer: gui::GuiRenderer
}

impl State {
    async fn new(window: Window) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = unsafe { instance.create_surface(&window) }.unwrap();

        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: Default::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false
        }).await.unwrap();

        let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
            features: wgpu::Features::empty(),
            limits: if cfg!(target_arch = "wasm32") {
                wgpu::Limits::downlevel_webgl2_defaults()
            } else {
                Default::default()
            },
            label: None
        }, None).await.unwrap();

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
            view_formats: vec![]
        };
        surface.configure(&device, &config);

        let scene = scene::Scene::new(&device, &queue, &config);

        let indicator_renderer = renderer::indicator::IndicatorRenderer::new(&device, config.format);

        let gui_renderer = gui::GuiRenderer::new(&window, &device, &config);

        Self {
            window,
            size,
            surface,
            device,
            queue,
            config,
            scene,
            indicator_renderer,
            gui_renderer
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);

            self.scene.resize(&self.device, &self.config, new_size.width, new_size.height);
        }
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        if let WindowEvent::KeyboardInput {
            input: KeyboardInput {
                state,
                virtual_keycode: Some(code),
                ..
            },
            ..
        } = event {
            if self.scene.handle_key(&self.device, &self.queue, *state == ElementState::Pressed, *code) { return true; }
        }

        self.gui_renderer.handle_events(event)
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&Default::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("[demo] Render Encoder")
        });

        self.scene.render(&self.queue, &mut encoder, &view);

        self.indicator_renderer.render(&mut encoder, &view);

        self.gui_renderer.render(&self.window, &self.device, &self.queue, &self.config, &mut encoder, &view);

        self.queue.submit(Some(encoder.finish()));
        output.present();

        Ok(())
    }
}

#[cfg_attr(target_arch="wasm32", wasm_bindgen(start))]
pub async fn run() {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Warn).unwrap();
        } else {
            env_logger::init();
        }
    }

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::WindowExtWebSys;

        window.set_inner_size(winit::dpi::PhysicalSize::new(800, 600));

        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| doc.body())
            .and_then(|body| {
                let canvas = web_sys::Element::from(window.canvas());
                canvas.set_attribute("autofocus", "").ok()?;
                body.append_child(&canvas).ok()?;
                Some(())
            })
            .unwrap();
    }

    let mut state = State::new(window).await;

    event_loop.run(move |event, _, control_flow| match event {
        Event::DeviceEvent {
            event: DeviceEvent::MouseMotion { delta },
            ..
        } => state.scene.handle_mouse_move(delta.0, delta.1),
        Event::WindowEvent {
            window_id,
            event
        } if window_id == state.window.id() => if !state.input(&event) {
            match event {
                WindowEvent::CloseRequested | WindowEvent::KeyboardInput {
                    input: KeyboardInput {
                        state: ElementState::Pressed,
                        virtual_keycode: Some(VirtualKeyCode::Escape),
                        ..
                    },
                    ..
                } => *control_flow = ControlFlow::Exit,
                WindowEvent::Resized(physical_size) =>
                    state.resize(physical_size),
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } =>
                    state.resize(*new_inner_size),
                _ => {}
            }
        }
        Event::RedrawRequested(window_id) if window_id == state.window.id() => {
            match state.render() {
                Ok(_) => {}
                Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                Err(e) => eprintln!("{:?}", e)
            }
        }
        Event::MainEventsCleared => state.window.request_redraw(),
        _ => {}
    });
}
