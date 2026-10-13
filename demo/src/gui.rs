pub struct GuiRenderer {
    context: egui::Context,
    renderer: egui_wgpu::Renderer,
    #[cfg(not(target_arch = "wasm32"))]
    state: egui_winit::State
}

impl GuiRenderer {
    pub fn new(#[allow(unused)] window: &winit::window::Window, device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> Self {
        let context = egui::Context::default();

        let renderer = egui_wgpu::Renderer::new(
            device,
            config.format,
            Default::default()
        );

        #[cfg(not(target_arch = "wasm32"))]
        let window_state = egui_winit::State::new(
            context.clone(),
            context.viewport_id(),
            window,
            None,
            None,
            None
        );

        Self {
            context,
            renderer,
            #[cfg(not(target_arch = "wasm32"))]
            state: window_state
        }
    }

    pub fn handle_events(&mut self, #[allow(unused)] window: &winit::window::Window, #[allow(unused)] event: &winit::event::WindowEvent) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        { self.state.on_window_event(window, event).consumed }

        #[cfg(target_arch = "wasm32")]
        false
    }

    pub fn render(&mut self, #[allow(unused)] window: &winit::window::Window, device: &wgpu::Device, queue: &wgpu::Queue, config: &wgpu::SurfaceConfiguration, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [config.width, config.height],
            pixels_per_point: self.context.pixels_per_point()
        };

        #[cfg(not(target_arch = "wasm32"))]
        let raw_input = self.state.take_egui_input(window);

        #[cfg(target_arch = "wasm32")]
        let raw_input = Default::default();

        let mut full_output = self.context.run_ui(raw_input, |ctx| {
            egui::Panel::left("My Panel").show(ctx, |ui| {
                ui.label("Hello egui!");
            });
        });

        #[cfg(not(target_arch = "wasm32"))]
        self.state.handle_platform_output(window, full_output.platform_output);

        let tris = self.context.tessellate(full_output.shapes, full_output.pixels_per_point);
        for (id, image_deltas) in &full_output.textures_delta.set {
            for image_delta in image_deltas {
                self.renderer.update_texture(device, queue, *id, image_delta);
            }
        }
        self.renderer.update_buffers(device, queue, encoder, &tris, &screen_descriptor);

        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("[egui] Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store
                }
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None
        }).forget_lifetime();
        self.renderer.render(&mut rpass, &tris, &screen_descriptor);

        for id in &full_output.textures_delta.free {
            self.renderer.free_texture(id);
        }
        full_output.textures_delta.clear();
    }
}
