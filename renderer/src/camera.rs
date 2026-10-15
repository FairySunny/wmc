pub struct CameraParameters {
    pub position: cgmath::Point3<f32>,
    pub yaw: cgmath::Deg<f32>,
    pub pitch: cgmath::Deg<f32>,
    pub fovy: cgmath::Deg<f32>,
    pub aspect: f32
}

impl CameraParameters {
    const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 0.5, 0.0,
        0.0, 0.0, 0.5, 1.0,
    );

    pub fn get_facing(&self) -> cgmath::Vector3<f32> {
        let (sin_yaw, cos_yaw) = cgmath::Angle::sin_cos(self.yaw);
        let (sin_pitch, cos_pitch) = cgmath::Angle::sin_cos(self.pitch);

        cgmath::vec3(
            cos_yaw * cos_pitch,
            sin_pitch,
            sin_yaw * cos_pitch
        )
    }

    pub fn get_view_proj_mat(&self) -> cgmath::Matrix4<f32> {
        let facing = self.get_facing();

        let view_mat = cgmath::Matrix4::look_to_rh(self.position, facing, cgmath::Vector3::unit_y());

        let proj_mat = cgmath::perspective(self.fovy, self.aspect, 0.1, 100.0);

        return Self::OPENGL_TO_WGPU_MATRIX * proj_mat * view_mat;
    }
}

pub struct Camera {
    pub parameters: CameraParameters,
    buffer: wgpu::Buffer,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup
}

impl Camera {
    pub fn new(device: &wgpu::Device, parameters: CameraParameters) -> Camera {
        let buffer = wgpu::util::DeviceExt::create_buffer_init(device, &wgpu::util::BufferInitDescriptor {
            label: Some("Camera (Uniform)"),
            contents: bytemuck::cast_slice(AsRef::<[f32; _]>::as_ref(&parameters.get_view_proj_mat())),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Camera (Uniform)"),
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

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera (Uniform)"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding()
            }]
        });

        Camera {
            parameters,
            buffer,
            bind_group_layout,
            bind_group
        }
    }

    pub fn update(&self, queue: &wgpu::Queue) {
        queue.write_buffer(
            &self.buffer,
            0,
            bytemuck::cast_slice(AsRef::<[f32; _]>::as_ref(&self.parameters.get_view_proj_mat()))
        );
    }

    pub fn get_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    pub fn get_bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}
