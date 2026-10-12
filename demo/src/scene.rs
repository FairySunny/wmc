pub struct Scene {
    depth_texture_view: wgpu::TextureView,
    model: crate::model::Model,
    texture_bind_group: wgpu::BindGroup,
    camera: renderer::camera::Camera,
    camera_buffer: wgpu::Buffer,
    camera_control: crate::control::CameraControl,
    camera_bind_group: wgpu::BindGroup,
    render_pipeline: wgpu::RenderPipeline
}

impl Scene {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, config: &wgpu::SurfaceConfiguration) -> Self {
        let depth_texture_view = crate::texture::create_depth_texture(device, config, "[demo] Depth Texture");

        let model = crate::model::Model::new(device, queue);

        let texture_bytes = include_bytes!("texture.png");
        let texture_image = crate::texture::Image::from_bytes(texture_bytes).unwrap();

        let texture_bind_group_layout = renderer::terrain::Scene::texture_bind_group_layout(device);

        let texture_bind_group = renderer::terrain::Scene::create_texture(
            device,
            queue,
            &texture_bind_group_layout,
            texture_image.width,
            texture_image.height,
            &texture_image.data
        );

        let camera = renderer::camera::Camera {
            pos: (5.0, 5.0, 5.0).into(),
            yaw: cgmath::Deg(-135.0),
            pitch: cgmath::Deg(-45.0),
            fovy: cgmath::Deg(45.0),
            aspect: config.width as f32 / config.height as f32
        };

        let camera_buffer = wgpu::util::DeviceExt::create_buffer_init(device, &wgpu::util::BufferInitDescriptor {
            label: Some("[demo] Camera Buffer"),
            contents: bytemuck::cast_slice(&[renderer::camera::CameraUniform::new(&camera)]),
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

        let camera_control = crate::control::CameraControl::new();

        let shader = device.create_shader_module(wgpu::include_wgsl!("../../renderer/src/shaders/terrain.wgsl"));

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("[demo] Render Pipeline Layout"),
            bind_group_layouts: &[
                &texture_bind_group_layout,
                &camera_bind_group_layout
            ],
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
                format: crate::texture::DEPTH_FORMAT,
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
            depth_texture_view,
            model,
            texture_bind_group,
            camera,
            camera_buffer,
            camera_control,
            camera_bind_group,
            render_pipeline
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, config: &wgpu::SurfaceConfiguration, width: u32, height: u32) {
        self.depth_texture_view = crate::texture::create_depth_texture(device, config, "[demo] Depth Texture");

        self.camera.aspect = width as f32 / height as f32;
    }

    pub fn handle_key(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, down: bool, code: winit::event::VirtualKeyCode) -> bool {
        if self.camera_control.handle_key(down, code) { return true; }

        if self.model.handle_key(device, queue, down, code) { return true; }

        return false;
    }

    pub fn handle_mouse_move(&mut self, x: f64, y: f64) {
        self.camera_control.handle_mouse_move(x, y);
    }

    pub fn render(&mut self, queue: &wgpu::Queue, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        self.camera_control.update_camera(&mut self.camera);
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[renderer::camera::CameraUniform::new(&self.camera)]));

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("[demo] Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
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
        render_pass.set_bind_group(0, &self.texture_bind_group, &[]);
        render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.model.buffer().slice(..));
        render_pass.draw(0..6, 0..self.model.len() as u32);
    }
}
