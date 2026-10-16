pub struct DynamicBuffer {
    label: String,
    buffer: wgpu::Buffer
}

impl DynamicBuffer {
    pub fn new(
        device: &wgpu::Device,
        label: String,
        initial_size: wgpu::BufferAddress,
        usage: wgpu::BufferUsages
    ) -> Self {
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
        log::debug!("DynamicBuffer ({}): {} / {}", self.label, data_size, old_size);

        if data_size > old_size {
            let mut new_size = old_size;
            while new_size < data_size { new_size *= 2; }
            log::debug!("DynamicBuffer ({}): {} -> {}", self.label, old_size, new_size);

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

pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

pub fn new_depth_texture(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Depth"),
        size: wgpu::Extent3d {
            width: config.width.max(1),
            height: config.height.max(1),
            depth_or_array_layers: 1
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[]
    });

    texture.create_view(&wgpu::TextureViewDescriptor {
        label: Some("Depth"),
        ..Default::default()
    })
}

pub fn generate_mipmaps(device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture) {
    let mip_cnt = texture.mip_level_count();

    let shader = device.create_shader_module(wgpu::include_wgsl!("shaders/mipmap.wgsl"));

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Mipmap"),
        layout: None,
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[]
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: Default::default(),
            targets: &[Some(texture.format().into())]
        }),
        multiview_mask: None,
        cache: None
    });

    let views: Vec<_> = (0..mip_cnt).map(|level| texture.create_view(&wgpu::TextureViewDescriptor {
        label: Some(&format!("Mipmap (Level {level})")),
        base_mip_level: level,
        mip_level_count: Some(1),
        ..Default::default()
    })).collect();

    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("Mipmap"),
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Mipmap")
    });

    for level in 1..mip_cnt as usize {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("Mipmap (Level {level})")),
            layout: &pipeline.get_bind_group_layout(0),
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
            label: Some(&format!("Mipmap (Level {level})")),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &views[level],
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
        });

        render_pass.set_pipeline(&pipeline);
        render_pass.set_bind_group(0, &bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }

    queue.submit([encoder.finish()]);
}
