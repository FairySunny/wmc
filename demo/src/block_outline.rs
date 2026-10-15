pub struct BlockOutline {
    render_pipeline: wgpu::RenderPipeline,
    position_buffer: wgpu::Buffer,
    should_render: bool,
    position_bind_group: wgpu::BindGroup
}

impl BlockOutline {
    pub fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration, camera_bind_group_layout: &wgpu::BindGroupLayout) -> BlockOutline {
        let position_buffer = wgpu::util::DeviceExt::create_buffer_init(device, &wgpu::util::BufferInitDescriptor {
            label: Some("[demo block_outline] Position Buffer"),
            contents: bytemuck::cast_slice(&[0i32; 3]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST
        });

        let position_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("[demo block_outline] Position Bind Group Layout"),
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

        let position_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("[demo block_outline] Position Bind Group"),
            layout: &position_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: position_buffer.as_entire_binding()
            }]
        });

        let shader = device.create_shader_module(wgpu::include_wgsl!("block_outline.wgsl"));

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("[demo block_outline] Render Pipeline Layout"),
            bind_group_layouts: &[
                Some(camera_bind_group_layout),
                Some(&position_bind_group_layout)
            ],
            immediate_size: 0
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("[demo block_outline] Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[]
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Line,
                conservative: false
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: crate::texture::DEPTH_FORMAT,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: Default::default(),
                bias: Default::default()
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false
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
            multiview_mask: None,
            cache: None
        });

        BlockOutline {
            render_pipeline,
            position_buffer,
            should_render: false,
            position_bind_group
        }
    }

    pub fn update(&mut self, queue: &wgpu::Queue, position: Option<[i32; 3]>) {
        if let Some(position) = position {
            queue.write_buffer(&self.position_buffer, 0, bytemuck::cast_slice(&position));
        }
        self.should_render = position.is_some();
    }

    pub fn render(&self, camera_bind_group: &wgpu::BindGroup, render_pass: &mut wgpu::RenderPass) {
        if !self.should_render {
            return;
        }

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, camera_bind_group, &[]);
        render_pass.set_bind_group(1, &self.position_bind_group, &[]);
        render_pass.draw(0..24, 0..1);
    }
}
