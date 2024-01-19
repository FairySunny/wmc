mod texture;
mod camera;

use wgpu::util::DeviceExt;
use winit::{
    event_loop::{EventLoop, ControlFlow},
    window::{WindowBuilder, Window},
    event::{Event, WindowEvent, KeyboardInput, ElementState, VirtualKeyCode}
};

pub struct SimpleChunk {
    data: Box<[u32; 100 * 16 * 16]>,
    updated: Vec<[i32; 3]>
}

impl SimpleChunk {
    pub fn new() -> Self {
        Self {
            data: Box::new([0; 100 * 16 * 16]),
            updated: vec![]
        }
    }

    fn index(coord: &[usize; 3]) -> usize {
        coord[1] * 16 * 16 + coord[0] * 16 + coord[2]
    }

    pub fn get(&self, coord: &[usize; 3]) -> u32 {
        self.data[Self::index(coord)]
    }

    pub fn get_mut(&mut self, coord: &[usize; 3]) -> &mut u32 {
        &mut self.data[Self::index(coord)]
    }

    pub fn update(&mut self, list: &[([usize; 3], u32)]) {
        let mut updated = vec![];
        for (coord, new_block) in list {
            let block = self.get_mut(coord);
            if *block == 0 && *new_block != 0 || *block != 0 && *new_block == 0 {
                updated.push([coord[0] as i32, coord[1] as i32, coord[2] as i32]);
            }
            *block = *new_block;
        }
        self.updated = updated;
    }
}

impl renderer::terrain::WorldInterface for SimpleChunk {
    fn get_block(&self, coord: &[i32; 3]) -> Option<u32> {
        let coord = [coord[0] as usize, coord[1] as usize, coord[2] as usize];
        let new = self.get(&coord);
        if new == 0 { None } else { Some(new) }
    }

    fn get_updated_block_coords(&self) -> &[[i32; 3]] {
        &self.updated
    }
}

struct State {
    window: Window,
    size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    depth_texture_view: wgpu::TextureView,
    chunk: SimpleChunk,
    i: usize,
    scene: renderer::terrain::Scene,
    scene_buffer: wgpu::Buffer,
    camera: camera::Camera,
    camera_buffer: wgpu::Buffer,
    camera_control: camera::CameraControl,
    camera_bind_group: wgpu::BindGroup,
    render_pipeline: wgpu::RenderPipeline
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
            limits: Default::default(),
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

        let depth_texture_view = texture::create_depth_texture(&device, &config, "[demo] Depth Texture");

        let mut chunk = SimpleChunk::new();
        chunk.update(&[
            ([1, 1, 1], 1),
            ([1, 1, 2], 1),
            ([2, 1, 1], 1),
            ([2, 1, 2], 1)
        ]);
        let mut scene = renderer::terrain::Scene::new();
        scene.update(&chunk);
        let scene_buffer = scene.create_buffer(&device);

        let camera = camera::Camera {
            pos: (4.0, 3.0, 4.0).into(),
            yaw: cgmath::Deg(-135.0),
            pitch: cgmath::Deg(-15.0),
            speed: 1.0,
            rot_speed: cgmath::Deg(70.0),
            fovy: cgmath::Deg(45.0).into(),
            aspect: config.width as f32 / config.height as f32,
            near: 0.1,
            far: 100.0
        };

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("[demo] Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera::CameraUniform::new(&camera)]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST
        });

        let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("[demo] Camera Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None
                },
                count: None
            }]
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("[demo] Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding()
            }]
        });

        let camera_control = camera::CameraControl::new();

        let shader = device.create_shader_module(wgpu::include_wgsl!("../../renderer/src/shaders/terrain.wgsl"));

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("[demo] Render Pipeline Layout"),
            bind_group_layouts: &[&camera_bind_group_layout],
            push_constant_ranges: &[]
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("[demo] Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[renderer::terrain::Scene::buffer_layout::<0>()]
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL
                })]
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: Default::default(),
                bias: Default::default()
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false
            },
            multiview: None
        });

        Self {
            window,
            size,
            surface,
            device,
            queue,
            config,
            depth_texture_view,
            chunk,
            i: 2,
            scene,
            scene_buffer,
            camera,
            camera_buffer,
            camera_bind_group,
            camera_control,
            render_pipeline
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);

            self.depth_texture_view = texture::create_depth_texture(&self.device, &self.config, "[demo] Depth Texture");

            self.camera.aspect = new_size.width as f32 / new_size.height as f32;
        }
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        if let winit::event::WindowEvent::KeyboardInput {
            input: winit::event::KeyboardInput {
                state: winit::event::ElementState::Released,
                virtual_keycode: Some(code),
                ..
            },
            ..
        } = event {
            match code {
                winit::event::VirtualKeyCode::Key1 => {
                    if self.i > 10 { return true; }
                    self.chunk.update(&[
                        ([1, self.i, 1], 1),
                        ([1, self.i, 2], 1),
                        ([2, self.i, 1], 1),
                        ([2, self.i, 2], 1)
                    ]);
                    self.scene.update(&self.chunk);
                    self.scene_buffer = self.scene.create_buffer(&self.device);
                    self.i += 1;
                    return true;
                }
                winit::event::VirtualKeyCode::Key2 => {
                    if self.i <= 2 { return true; }
                    self.i -= 1;
                    self.chunk.update(&[
                        ([1, self.i, 1], 0),
                        ([1, self.i, 2], 0),
                        ([2, self.i, 1], 0),
                        ([2, self.i, 2], 0)
                    ]);
                    self.scene.update(&self.chunk);
                    self.scene_buffer = self.scene.create_buffer(&self.device);
                    return true;
                }
                _ => {}
            }
        }

        self.camera_control.handle_events(event)
    }

    fn update(&mut self) {
        self.camera_control.update_camera(&mut self.camera);
        self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[camera::CameraUniform::new(&self.camera)]));
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&Default::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("[demo] Render Encoder")
        });

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("[demo] Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.1, g: 0.2, b: 0.3, a: 1.0 }),
                    store: wgpu::StoreOp::Store
                }
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_texture_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store
                }),
                stencil_ops: None
            }),
            occlusion_query_set: None,
            timestamp_writes: None
        });

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.scene_buffer.slice(..));
        render_pass.draw(0..6, 0..self.scene.len() as u32);

        drop(render_pass);

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

pub async fn run() {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let mut state = State::new(window).await;

    event_loop.run(move |event, _, control_flow| match event {
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
            state.update();
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
