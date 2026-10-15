pub struct Scene {
    depth_texture_view: wgpu::TextureView,
    model: crate::model::Model,
    texture_bind_group: wgpu::BindGroup,
    camera: renderer::camera::Camera,
    camera_control: crate::control::CameraControl,
    render_pipeline: wgpu::RenderPipeline,
    #[cfg(not(target_arch = "wasm32"))]
    block_outline: crate::block_outline::BlockOutline,
    pub debug_target: Option<[i32; 3]>
}

impl Scene {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, config: &wgpu::SurfaceConfiguration) -> Self {
        let depth_texture_view = crate::texture::create_depth_texture(device, config, "[demo] Depth Texture");

        let model = crate::model::Model::new(device, queue);

        let texture_bytes = include_bytes!("texture.png");
        let texture_image = crate::texture::Image::from_bytes(texture_bytes).unwrap();

        let texture_bind_group_layout = renderer::terrain::Terrain::texture_bind_group_layout(device);

        let texture_bind_group = renderer::terrain::Terrain::create_texture(
            device,
            queue,
            &texture_bind_group_layout,
            texture_image.width,
            texture_image.height,
            &texture_image.data
        );

        let camera = renderer::camera::Camera::new(device, renderer::camera::CameraParameters {
            position: (5.0, 5.0, 5.0).into(),
            yaw: cgmath::Deg(-135.0),
            pitch: cgmath::Deg(-45.0),
            fovy: cgmath::Deg(45.0),
            aspect: config.width as f32 / config.height as f32
        });

        let camera_control = crate::control::CameraControl::new();

        let shader = device.create_shader_module(wgpu::include_wgsl!("../../renderer/src/shaders/terrain.wgsl"));

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("[demo] Render Pipeline Layout"),
            bind_group_layouts: &[
                Some(&texture_bind_group_layout),
                Some(camera.get_bind_group_layout())
            ],
            immediate_size: 0
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("[demo] Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(renderer::terrain::Terrain::buffer_layout::<0>())]
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
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
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default()
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false
            },
            multiview_mask: None,
            cache: None
        });

        #[cfg(not(target_arch = "wasm32"))]
        let block_outline = crate::block_outline::BlockOutline::new(device, config, camera.get_bind_group_layout());

        Self {
            depth_texture_view,
            model,
            texture_bind_group,
            camera,
            camera_control,
            render_pipeline,
            #[cfg(not(target_arch = "wasm32"))]
            block_outline,
            debug_target: None
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, config: &wgpu::SurfaceConfiguration, width: u32, height: u32) {
        self.depth_texture_view = crate::texture::create_depth_texture(device, config, "[demo] Depth Texture");

        self.camera.parameters.aspect = width as f32 / height as f32;
    }

    pub fn handle_key(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, down: bool, code: winit::keyboard::KeyCode) -> bool {
        if self.camera_control.handle_key(down, code) { return true; }

        if self.model.handle_key(device, queue, down, code) { return true; }

        return false;
    }

    pub fn handle_mouse_move(&mut self, x: f64, y: f64) {
        self.camera_control.handle_mouse_move(x, y);
    }

    pub fn update(&mut self, queue: &wgpu::Queue) {
        self.camera_control.update_camera_parameters(&mut self.camera.parameters);
        self.camera.update(queue);

        let block_outline_position = self.model.intersect(self.camera.parameters.position.as_ref(), (self.camera.parameters.position + self.camera.parameters.get_facing() * 10.0).as_ref());
        self.debug_target = block_outline_position;

        #[cfg(not(target_arch = "wasm32"))]
        self.block_outline.update(queue, block_outline_position);
    }

    pub fn render(&mut self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("[demo] Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
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
            timestamp_writes: None,
            multiview_mask: None
        });

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.texture_bind_group, &[]);
        render_pass.set_bind_group(1, self.camera.get_bind_group(), &[]);
        render_pass.set_vertex_buffer(0, self.model.buffer().slice(..));
        render_pass.draw(0..6, 0..self.model.len());

        #[cfg(not(target_arch = "wasm32"))]
        self.block_outline.render(self.camera.get_bind_group(), &mut render_pass);
    }
}
