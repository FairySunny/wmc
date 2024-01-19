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
