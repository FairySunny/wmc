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

pub struct Terrain {
    faces: Vec<Face>,
    buffer: crate::util::DynamicBuffer
}

impl Terrain {
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

    pub fn update(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, world: &impl TerrainUpdate) {
        let reserved = self.faces.drain(..).filter_map(|face| {
            let position = BlockPosition(face.position);
            if world.is_updated(&position) {
                return None;
            }

            let facing_position = position.next(Direction(face.direction as u8));
            if world.is_updated(&facing_position) {
                None
            } else {
                Some(face)
            }
        });

        let added = world.get_updated_positions().iter().flat_map(|position| {
            let block = world.get_block(position);

            (0..6).filter_map(move |direction_id| {
                let direction = Direction(direction_id as u8);
                let opposite = direction.opposite();

                let facing_position = position.next(direction);
                let facing_block = world.get_block(&facing_position);

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

    pub fn len(&self) -> u32 {
        self.faces.len() as u32
    }

    pub fn buffer(&self) -> &wgpu::Buffer {
        self.buffer.buffer()
    }

    pub fn buffer_layout<const LOCATION_OFFSET: u32>() -> wgpu::VertexBufferLayout<'static> {
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

    pub fn texture_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
        })
    }

    pub fn create_texture(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        layout: &wgpu::BindGroupLayout,
        width: u32,
        height: u32,
        data: &[u8]
    ) -> wgpu::BindGroup {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Terrain (Texture)"),
            size,
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
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height)
            },
            size
        );

        crate::util::generate_mipmaps(device, queue, &texture);

        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("Terrain (Texture)"),
            ..Default::default()
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Terrain (Texture)"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });

        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Terrain (Texture)"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view)
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler)
                }
            ]
        })
    }
}
