fn opposite_direction(direction: u32) -> u32 {
    direction ^ 1
}

fn next_coord(coord: &[i32; 3], direction: u32) -> [i32; 3] {
    let mut next = *coord;
    next[(direction >> 1) as usize] += ((direction & 1) << 1) as i32 - 1;
    next
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
struct Face {
    coord: [i32; 3],
    direction: u32,
    texture: u32
}

pub struct Scene {
    faces: Vec<Face>,
    buffer: crate::utils::DynamicBuffer
}

pub trait WorldInterface {
    fn get_block(&self, coord: &[i32; 3]) -> Option<u32>;
    fn get_updated_block_coords(&self) -> &[[i32; 3]];
}

impl Scene {
    pub fn new(device: &wgpu::Device) -> Self {
        Self {
            faces: vec![],
            buffer: crate::utils::DynamicBuffer::new(
                device,
                "[terrain] Face Instance Buffer".into(),
                512,
                wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST
            )
        }
    }

    pub fn update(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, world: &impl WorldInterface) {
        let reserved = self.faces.iter().filter_map(|face| {
            let block = world.get_block(&face.coord);
            let new_texture = match block {
                Some(t) => t,
                None => return None
            };

            let facing_coord = next_coord(&face.coord, face.direction);
            let facing_block = world.get_block(&facing_coord);
            match facing_block {
                Some(_) => None,
                None => Some(Face { texture: new_texture, ..*face })
            }
        });

        let added = world.get_updated_block_coords().iter().flat_map(|coord| {
            let block = world.get_block(coord);

            (0..6).filter_map(move |direction| {
                let facing_coord = next_coord(coord, direction);
                let facing_block = world.get_block(&facing_coord);
                match block {
                    Some(texture) => match facing_block {
                        Some(_) => None,
                        None => Some(Face { coord: *coord, direction, texture })
                    }
                    None => match facing_block {
                        Some(texture) => Some(Face {
                            coord: facing_coord,
                            direction: opposite_direction(direction),
                            texture
                        }),
                        None => None
                    }
                }
            })
        });

        self.faces = reserved.chain(added).collect();

        self.buffer.update(device, queue, bytemuck::cast_slice(&self.faces));
    }

    pub fn len(&self) -> usize {
        self.faces.len()
    }

    pub fn buffer(&self) -> &wgpu::Buffer {
        self.buffer.buffer()
    }

    pub fn buffer_layout<const N: u32>() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Face>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: N,
                    format: wgpu::VertexFormat::Sint32x3
                },
                wgpu::VertexAttribute {
                    offset: 12,
                    shader_location: N + 1,
                    format: wgpu::VertexFormat::Uint32
                },
                wgpu::VertexAttribute {
                    offset: 16,
                    shader_location: N + 2,
                    format: wgpu::VertexFormat::Uint32
                }
            ]
        }
    }

    pub fn texture_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("[terrain] Texture Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true }
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

    pub fn create_texture(device: &wgpu::Device, queue: &wgpu::Queue, layout: &wgpu::BindGroupLayout, width: u32, height: u32, data: &[u8]) -> wgpu::BindGroup {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("[terrain] Texture"),
            size,
            mip_level_count: 5,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING |
                wgpu::TextureUsages::COPY_DST |
                wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[]
        });

        queue.write_texture(wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All
        }, data, wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(4 * width),
            rows_per_image: Some(height)
        }, size);

        crate::utils::generate_mipmaps(device, queue, &texture);

        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("[terrain] Texture View"),
            ..Default::default()
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("[terrain] Texture Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("[terrain] Texture Bind Group"),
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
