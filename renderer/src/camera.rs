pub struct Camera {
    pub pos: cgmath::Point3<f32>,
    pub yaw: cgmath::Deg<f32>,
    pub pitch: cgmath::Deg<f32>,
    pub fovy: cgmath::Deg<f32>,
    pub aspect: f32
}

impl Camera {
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

        let view_mat = cgmath::Matrix4::look_to_rh(self.pos, facing, cgmath::Vector3::unit_y());

        let proj_mat = cgmath::perspective(self.fovy, self.aspect, 0.1, 100.0);

        return Self::OPENGL_TO_WGPU_MATRIX * proj_mat * view_mat;
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
pub struct CameraUniform {
    view_proj_mat: [[f32; 4]; 4]
}

impl CameraUniform {
    pub fn new(camera: &Camera) -> Self {
        Self {
            view_proj_mat: camera.get_view_proj_mat().into()
        }
    }
}
