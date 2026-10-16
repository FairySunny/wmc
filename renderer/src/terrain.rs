#[repr(C)]
#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
struct Face {
    position: [i32; 3],
    direction: u32,
    texture_id: u32
}

#[derive(Clone, Copy)]
pub struct Direction(pub u8); // TODO: Check range

impl Direction {
    pub fn opposite(self) -> Self {
        Self(self.0 ^ 1)
    }
}

pub struct BlockPosition(pub [i32; 3]);

impl BlockPosition {
    pub fn next(&self, direction: Direction) -> Self {
        let mut coords = self.0;
        coords[(direction.0 >> 1) as usize] += ((direction.0 & 1) << 1) as i32 - 1;
        Self(coords)
    }
}

pub type TextureId = std::num::NonZeroU8;

pub struct Block {
    pub faces: [Option<TextureId>; 6]
}

impl Block {
    pub fn face(&self, direction: Direction) -> Option<TextureId> {
        self.faces[direction.0 as usize]
    }
}

pub trait TerrainUpdate {
    fn is_updated(&self, position: &BlockPosition) -> bool;
    fn get_updated_positions(&self) -> &[BlockPosition];
    fn get_block(&self, position: &BlockPosition) -> &Block;
}

pub struct TerrainModel {
    faces: Vec<Face>,
    buffer: crate::util::DynamicBuffer
}

impl TerrainModel {
    pub fn new(device: &wgpu::Device) -> Self {
        Self {
            faces: vec![],
            buffer: crate::util::DynamicBuffer::new(
                device,
                "Terrain (Instance)".into(),
                512,
                wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::VERTEX
            )
        }
    }

    pub fn update(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, update: &impl TerrainUpdate) {
        let reserved = self.faces.drain(..).filter_map(|face| {
            let position = BlockPosition(face.position);
            if update.is_updated(&position) {
                return None;
            }

            let facing_position = position.next(Direction(face.direction as u8));
            if update.is_updated(&facing_position) {
                None
            } else {
                Some(face)
            }
        });

        let added = update.get_updated_positions().iter().flat_map(|position| {
            let block = update.get_block(position);

            (0..6).filter_map(move |direction_id| {
                let direction = Direction(direction_id as u8);
                let opposite = direction.opposite();

                let facing_position = position.next(direction);
                let facing_block = update.get_block(&facing_position);

                let face = block.face(direction);
                let facing_face = facing_block.face(opposite);

                match face {
                    Some(texture_id) => match facing_face {
                        Some(_) => None,
                        None => Some(Face {
                            position: position.0,
                            direction: direction_id,
                            texture_id: texture_id.get().into()
                        })
                    }
                    None => match facing_face {
                        Some(texture_id) => Some(Face {
                            position: facing_position.0,
                            direction: opposite.0.into(),
                            texture_id: texture_id.get().into()
                        }),
                        None => None
                    }
                }
            })
        });

        self.faces = reserved.chain(added).collect();

        self.buffer.update(device, queue, bytemuck::cast_slice(&self.faces));
    }

    pub fn get_instance_buffer(&self) -> &wgpu::Buffer {
        self.buffer.buffer()
    }

    pub fn get_instance_count(&self) -> u32 {
        self.faces.len() as u32
    }

    pub const fn get_instance_buffer_layout<const LOCATION_OFFSET: u32>() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Face>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &const { wgpu::vertex_attr_array![
                LOCATION_OFFSET => Sint32x3,
                LOCATION_OFFSET + 1 => Uint32,
                LOCATION_OFFSET + 2 => Uint32
            ] }
        }
    }
}

pub struct TerrainRenderer {
    pipeline: wgpu::RenderPipeline,
    texture_bind_group: wgpu::BindGroup
}

impl TerrainRenderer {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        color_format: wgpu::TextureFormat,
        depth_stencil_format: wgpu::TextureFormat,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        texture_width: u32,
        texture_height: u32,
        texture_data: &[u8]
    ) -> Self {

        // texture

        let texture_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Terrain (Texture)"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false
                    },
                    count: None
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None
                }
            ]
        });

        let texture_size = wgpu::Extent3d {
            width: texture_width,
            height: texture_height,
            depth_or_array_layers: 1
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Terrain (Texture)"),
            size: texture_size,
            mip_level_count: 5,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[]
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All
            },
            texture_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(texture_width * 4),
                rows_per_image: Some(texture_height)
            },
            texture_size
        );

        crate::util::generate_mipmaps(device, queue, &texture);

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("Terrain (Texture)"),
            ..Default::default()
        });

        let texture_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Terrain (Texture)"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });

        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Terrain (Texture)"),
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view)
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture_sampler)
                }
            ]
        });

        // pipeline

        let shader = device.create_shader_module(wgpu::include_wgsl!("shaders/terrain.wgsl"));

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Terrain"),
            bind_group_layouts: &[
                Some(camera_bind_group_layout),
                Some(&texture_bind_group_layout)
            ],
            immediate_size: 0
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Terrain"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(TerrainModel::get_instance_buffer_layout::<0>())]
            },
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
                format: depth_stencil_format,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default()
            }),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL
                })]
            }),
            multiview_mask: None,
            cache: None
        });

        Self {
            pipeline,
            texture_bind_group
        }
    }

    pub fn render(
        &self,
        render_pass: &mut wgpu::RenderPass,
        camera_bind_group: &wgpu::BindGroup,
        model: &TerrainModel
    ) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, camera_bind_group, &[]);
        render_pass.set_bind_group(1, &self.texture_bind_group, &[]);
        render_pass.set_vertex_buffer(0, model.get_instance_buffer().slice(..));
        render_pass.draw(0..6, 0..model.get_instance_count());
    }
}
