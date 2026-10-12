use std::collections::HashSet;

pub struct SimpleChunk {
    data: Box<[u32; 100 * 16 * 16]>,
    updated: Vec<renderer::terrain::IntCoord>,
    updated_set: HashSet<[i32; 3]>
}

impl SimpleChunk {
    pub fn new() -> Self {
        Self {
            data: Box::new([0; 100 * 16 * 16]),
            updated: vec![],
            updated_set: HashSet::new()
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
        let mut updated_set = HashSet::new();
        for (coord, new_block) in list {
            let block = self.get_mut(coord);
            if block != new_block {
                let coord = [coord[0] as i32, coord[1] as i32, coord[2] as i32];
                updated.push(renderer::terrain::IntCoord(coord));
                updated_set.insert(coord);
                *block = *new_block;
            }
        }
        self.updated = updated;
        self.updated_set = updated_set;
    }
}

impl renderer::terrain::WorldInterface for SimpleChunk {
    fn get_block(&self, coord: &renderer::terrain::IntCoord) -> &renderer::terrain::BlockModel {
        const MODELS: &[renderer::terrain::BlockModel] = &[
            renderer::terrain::BlockModel {
                faces: [None; 6]
            },
            renderer::terrain::BlockModel {
                faces: [renderer::terrain::TextureId::new(1); 6]
            },
            renderer::terrain::BlockModel {
                faces: [renderer::terrain::TextureId::new(2); 6]
            }
        ];

        let coord = [coord.0[0] as usize, coord.0[1] as usize, coord.0[2] as usize];
        let new = self.get(&coord);
        &MODELS[new as usize]
    }

    fn is_updated(&self, coord: &renderer::terrain::IntCoord) -> bool {
        self.updated_set.contains(&coord.0)
    }

    fn get_updated_block_coords(&self) -> &[renderer::terrain::IntCoord] {
        &self.updated
    }
}

pub struct Model {
    chunk: SimpleChunk,
    i: usize,
    scene: renderer::terrain::Scene
}

impl Model {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let mut chunk = SimpleChunk::new();
        chunk.update(&[
            ([1, 1, 1], 1),
            ([1, 1, 2], 2),
            ([2, 1, 1], 2),
            ([2, 1, 2], 1)
        ]);

        let mut scene = renderer::terrain::Scene::new(&device);
        scene.update(&device, &queue, &chunk);

        Self {
            chunk,
            i: 2,
            scene
        }
    }

    pub fn handle_key(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, down: bool, code: winit::keyboard::KeyCode) -> bool {
        if !down { return false };

        match code {
            winit::keyboard::KeyCode::Digit1 => {
                if self.i > 10 { return true; }
                let i = self.i as u32;
                self.chunk.update(&[
                    ([1, self.i, 1], (i + 1) % 2 + 1),
                    ([1, self.i, 2], (i + 2) % 2 + 1),
                    ([2, self.i, 1], (i + 2) % 2 + 1),
                    ([2, self.i, 2], (i + 1) % 2 + 1)
                ]);
                self.scene.update(device, queue, &self.chunk);
                self.i += 1;
            }
            winit::keyboard::KeyCode::Digit2 => {
                if self.i <= 2 { return true; }
                self.i -= 1;
                self.chunk.update(&[
                    ([1, self.i, 1], 0),
                    ([1, self.i, 2], 0),
                    ([2, self.i, 1], 0),
                    ([2, self.i, 2], 0)
                ]);
                self.scene.update(device, queue, &self.chunk);
            }
            _ => return false
        }

        return true;
    }

    pub fn buffer(&self) -> &wgpu::Buffer {
        self.scene.buffer()
    }

    pub fn len(&self) -> usize {
        self.scene.len()
    }
}
