use std::collections::HashSet;

pub struct SimpleChunk {
    data: Box<[u32; 100 * 16 * 16]>,
    updated: Vec<renderer::terrain::BlockPosition>,
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
                updated.push(renderer::terrain::BlockPosition(coord));
                updated_set.insert(coord);
                *block = *new_block;
            }
        }
        self.updated = updated;
        self.updated_set = updated_set;
    }
}

impl renderer::terrain::TerrainUpdate for SimpleChunk {
    fn is_updated(&self, position: &renderer::terrain::BlockPosition) -> bool {
        self.updated_set.contains(&position.0)
    }

    fn get_updated_positions(&self) -> &[renderer::terrain::BlockPosition] {
        &self.updated
    }

    fn get_block(&self, position: &renderer::terrain::BlockPosition) -> &renderer::terrain::Block {
        const BLOCK_TYPES: &[renderer::terrain::Block] = &[
            renderer::terrain::Block {
                faces: [None; 6]
            },
            renderer::terrain::Block {
                faces: [renderer::terrain::TextureId::new(1); 6]
            },
            renderer::terrain::Block {
                faces: [renderer::terrain::TextureId::new(2); 6]
            }
        ];

        let coord = [position.0[0] as usize, position.0[1] as usize, position.0[2] as usize];
        let new = self.get(&coord);
        &BLOCK_TYPES[new as usize]
    }
}

pub struct Model {
    chunk: SimpleChunk,
    i: usize,
    terrain_model: renderer::terrain::TerrainModel
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

        let mut terrain_model = renderer::terrain::TerrainModel::new(&device);
        terrain_model.update(&device, &queue, &chunk);

        Self {
            chunk,
            i: 2,
            terrain_model
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
                self.terrain_model.update(device, queue, &self.chunk);
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
                self.terrain_model.update(device, queue, &self.chunk);
            }
            _ => return false
        }

        return true;
    }

    pub fn get_terrain_model(&self) -> &renderer::terrain::TerrainModel {
        &self.terrain_model
    }

    pub fn get(&self, coord: &[i32; 3]) -> u32 {
        self.chunk.get(&coord.map(|x| x as usize))
    }

    pub fn intersect(&self, from: &[f32; 3], to: &[f32; 3]) -> Option<[i32; 3]> {
        if from.iter().any(|x| !x.is_finite()) || from == to {
            return None;
        }

        let mut pos = from.map(|x| x.floor() as i32);

        if pos.iter().all(|&x| x >= 0) && pos[0] < 16 && pos[1] < 100 && pos[2] < 16
            && self.chunk.get(&pos.map(|x| x as usize)) > 0 {
            return Some(pos);
        }

        let d = [to[0] - from[0], to[1] - from[1], to[2] - from[2]];
        if d.iter().any(|x| !x.is_finite()) {
            return None;
        }

        let sign = d.map(|x| x.signum());
        let sign_i = sign.map(|x| x as i32);
        let t_delta = [sign[0] / d[0], sign[1] / d[1], sign[2] / d[2]].map(|x| x.min(f32::MAX));

        let mut t = [
            t_delta[0] * ((sign[0] + 1.0) / 2.0 - sign[0] * (from[0] - from[0].floor())),
            t_delta[1] * ((sign[1] + 1.0) / 2.0 - sign[1] * (from[1] - from[1].floor())),
            t_delta[2] * ((sign[2] + 1.0) / 2.0 - sign[2] * (from[2] - from[2].floor()))
        ];

        while t.iter().any(|&x| x <= 1.0) {
            let ((t_n, t_delta_n), (pos_n, sign_i_n)) = t.iter_mut().zip(t_delta)
                .zip(pos.iter_mut().zip(sign_i))
                .min_by(|((t1, _), _), ((t2, _), _)| t1.total_cmp(t2))
                .unwrap();
            *t_n += t_delta_n;
            *pos_n += sign_i_n;

            if pos.iter().all(|&x| x >= 0) && pos[0] < 16 && pos[1] < 100 && pos[2] < 16
                && self.chunk.get(&pos.map(|x| x as usize)) > 0 {
                return Some(pos);
            }
        }

        None
    }
}
