pub struct EguiState {
    context: egui::Context,
    state: egui_winit::State,
    renderer: egui_wgpu::Renderer
}

impl EguiState {
    pub fn new(window: &winit::window::Window, device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> Self {
        let context = egui::Context::default();

        let window_state = egui_winit::State::new(
            context.viewport_id(),
            window,
            None,
            None
        );

        let renderer = egui_wgpu::renderer::Renderer::new(
            device,
            config.format,
            None,
            1
        );

        Self {
            context,
            state: window_state,
            renderer
        }
    }

    pub fn handle_events(&mut self, event: &winit::event::WindowEvent) -> bool {
        self.state.on_window_event(&self.context, event).consumed
    }

    pub fn render(&mut self, window: &winit::window::Window, device: &wgpu::Device, queue: &wgpu::Queue, config: &wgpu::SurfaceConfiguration, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        let screen_descriptor = egui_wgpu::renderer::ScreenDescriptor {
            size_in_pixels: [config.width, config.height],
            pixels_per_point: self.context.pixels_per_point()
        };

        let raw_input = self.state.take_egui_input(window);
        let full_output = self.context.run(raw_input, |ctx| {
            egui::SidePanel::left("My Panel").show(ctx, |ui| {
                ui.label("Hello egui!");
            });
        });

        self.state.handle_platform_output(window, &self.context, full_output.platform_output);

        let tris = self.context.tessellate(full_output.shapes, full_output.pixels_per_point);
        for (id, image_delta) in &full_output.textures_delta.set {
            self.renderer.update_texture(device, queue, *id, image_delta);
        }
        self.renderer.update_buffers(device, queue, encoder, &tris, &screen_descriptor);

        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("[egui] Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store
                }
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None
        });
        self.renderer.render(&mut rpass, &tris, &screen_descriptor);
        drop(rpass);

        for x in &full_output.textures_delta.free {
            self.renderer.free_texture(x);
        }
    }
}
