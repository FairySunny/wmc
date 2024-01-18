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
    faces: Vec<Face>
}

pub trait WorldInterface {
    fn get_block(&self, coord: &[i32; 3]) -> Option<u32>;
    fn get_updated_block_coords(&self) -> &[[i32; 3]];
}

pub struct SimpleChunk {
    data: Box<[u32; 100 * 16 * 16]>,
    updated: Vec<[i32; 3]>
}

impl SimpleChunk {
    pub fn new() -> Self {
        Self {
            data: Box::new([0; 100 * 16 * 16]),
            updated: vec![]
        }
    }

    fn index(coord: &[usize; 3]) -> usize {
        coord[1] * 16 * 16 + coord[0] * 16 + coord[2]
    }

    pub fn get(&self, coord: &[usize; 3]) -> u32 {
        self.data[Self::index(coord)]
    }

    pub fn get_mut(&mut self, coord: &[usize; 3]) -> &mut u32 {
        &mut self.data[Self::index(coord)]
    }

    pub fn update(&mut self, list: &[([usize; 3], u32)]) {
        let mut updated = vec![];
        for (coord, new_block) in list {
            let block = self.get_mut(coord);
            if *block == 0 && *new_block != 0 || *block != 0 && *new_block == 0 {
                updated.push([coord[0] as i32, coord[1] as i32, coord[2] as i32]);
            }
            *block = *new_block;
        }
        self.updated = updated;
    }
}

impl WorldInterface for SimpleChunk {
    fn get_block(&self, coord: &[i32; 3]) -> Option<u32> {
        let coord = [coord[0] as usize, coord[1] as usize, coord[2] as usize];
        let new = self.get(&coord);
        if new == 0 { None } else { Some(new) }
    }

    fn get_updated_block_coords(&self) -> &[[i32; 3]] {
        &self.updated
    }
}

impl Scene {
    pub fn new() -> Self {
        Self { faces: vec![] }
    }

    pub fn update(&mut self, world: &impl WorldInterface) {
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

    pub fn create_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        wgpu::util::DeviceExt::create_buffer_init(device, &wgpu::util::BufferInitDescriptor {
            label: Some("Voxel Face Instance Buffer"),
            contents: bytemuck::cast_slice(&self.faces),
            usage: wgpu::BufferUsages::VERTEX
        })
    }

    pub fn len(&self) -> usize {
        self.faces.len()
    }
}
