pub struct DynamicBuffer {
    label: String,
    buffer: wgpu::Buffer
}

impl DynamicBuffer {
    pub fn new(device: &wgpu::Device, label: String, initial_size: wgpu::BufferAddress, usage: wgpu::BufferUsages) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&label),
            size: initial_size,
            usage,
            mapped_at_creation: false
        });

        Self { label, buffer }
    }

    pub fn update(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, data: &[u8]) {
        let data_size = data.len() as wgpu::BufferAddress;
        let old_size = self.buffer.size();
        eprintln!("{} / {}", data_size, old_size);
        if data_size > old_size {
            let mut new_size = old_size;
            while new_size < data_size { new_size *= 2; }
            eprintln!("{} -> {}", old_size, new_size);
            self.buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&self.label),
                size: new_size,
                usage: self.buffer.usage(),
                mapped_at_creation: false
            });
        }

        queue.write_buffer(&self.buffer, 0, data);
    }

    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }
}

pub fn generate_mipmaps(device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture) {
    let mip_cnt = texture.mip_level_count();

    let shader = device.create_shader_module(wgpu::include_wgsl!("shaders/mipmap.wgsl"));

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("[mipmap] Render Pipeline"),
        layout: None,
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[]
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(texture.format().into())]
        }),
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        multiview: None
    });

    let bind_group_layout = pipeline.get_bind_group_layout(0);

    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("[mipmap] Sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });

    let views: Vec<_> = (0..mip_cnt).map(|level| texture.create_view(&wgpu::TextureViewDescriptor {
        label: Some(&format!("[mipmap] View Level {level}")),
        format: None,
        dimension: None,
        aspect: wgpu::TextureAspect::All,
        base_mip_level: level,
        mip_level_count: Some(1),
        base_array_layer: 0,
        array_layer_count: None
    })).collect();

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("[mipmap] Render Encoder")
    });

    for level in 1..mip_cnt as usize {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("[mipmap] Texture Bind Group Level {level}")),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&views[level - 1])
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler)
                }
            ]
        });

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(&format!("[mipmap] Render Pass Level {level}")),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &views[level],
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store
                }
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None
        });

        render_pass.set_pipeline(&pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }

    queue.submit(Some(encoder.finish()));
}
