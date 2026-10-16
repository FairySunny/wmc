pub struct Scene {
    depth_texture_view: wgpu::TextureView,
    model: crate::model::Model,
    camera: renderer::camera::Camera,
    camera_control: crate::control::CameraControl,
    terrain_renderer: renderer::terrain::TerrainRenderer,
    #[cfg(not(target_arch = "wasm32"))]
    block_outline: crate::block_outline::BlockOutline,
    pub debug_target: Option<([i32; 3], u32)>
}

impl Scene {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, config: &wgpu::SurfaceConfiguration) -> Self {
        let depth_texture_view = crate::texture::create_depth_texture(device, config, "[demo] Depth Texture");

        let model = crate::model::Model::new(device, queue);

        let camera = renderer::camera::Camera::new(device, renderer::camera::CameraParameters {
            position: (5.0, 5.0, 5.0).into(),
            yaw: cgmath::Deg(-135.0),
            pitch: cgmath::Deg(-45.0),
            fovy: cgmath::Deg(45.0),
            aspect: config.width as f32 / config.height as f32
        });

        let camera_control = crate::control::CameraControl::new();

        let texture_image = crate::texture::Image::from_bytes(include_bytes!("texture.png")).unwrap();

        let terrain_renderer = renderer::terrain::TerrainRenderer::new(
            device,
            queue,
            config.format,
            crate::texture::DEPTH_FORMAT,
            camera.get_bind_group_layout(),
            texture_image.width,
            texture_image.height,
            &texture_image.data
        );

        #[cfg(not(target_arch = "wasm32"))]
        let block_outline = crate::block_outline::BlockOutline::new(device, config, camera.get_bind_group_layout());

        Self {
            depth_texture_view,
            model,
            camera,
            camera_control,
            terrain_renderer,
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
        self.debug_target = block_outline_position.map(|pos| (pos, self.model.get(&pos)));

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

        self.terrain_renderer.render(
            &mut render_pass,
            self.camera.get_bind_group(),
            self.model.get_terrain_model()
        );

        #[cfg(not(target_arch = "wasm32"))]
        self.block_outline.render(self.camera.get_bind_group(), &mut render_pass);
    }
}
