use cgmath::prelude::*;
use winit::event::{WindowEvent, KeyboardInput, VirtualKeyCode, ElementState};

pub struct Camera {
    pub pos: cgmath::Point3<f32>,
    pub yaw: cgmath::Deg<f32>,
    pub pitch: cgmath::Deg<f32>,

    pub speed: f32,
    pub rot_speed: cgmath::Deg<f32>,

    pub fovy: cgmath::Rad<f32>,
    pub aspect: f32,
    pub near: f32,
    pub far: f32
}

impl Camera {
    const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 0.5, 0.5,
        0.0, 0.0, 0.0, 1.0,
    );

    pub const CMD_FRONT    : u32 = 1 << 0;
    pub const CMD_BACK     : u32 = 1 << 1;
    pub const CMD_RIGHT    : u32 = 1 << 2;
    pub const CMD_LEFT     : u32 = 1 << 3;
    pub const CMD_UP       : u32 = 1 << 4;
    pub const CMD_DOWN     : u32 = 1 << 5;
    pub const CMD_ROT_RIGHT: u32 = 1 << 6;
    pub const CMD_ROT_LEFT : u32 = 1 << 7;
    pub const CMD_ROT_UP   : u32 = 1 << 8;
    pub const CMD_ROT_DOWN : u32 = 1 << 9;

    pub fn handle_cmds(&mut self, cmds: u32, time_span: f32, rot_right: f32, rot_up: f32) {
        let mov = self.speed * time_span;
        let rot = self.rot_speed * 500.0 * time_span;

        self.yaw += self.rot_speed * rot_right;
        self.pitch += self.rot_speed * rot_up;
        if cmds & Self::CMD_ROT_RIGHT != 0 { self.yaw += rot; }
        if cmds & Self::CMD_ROT_LEFT  != 0 { self.yaw -= rot; }
        if cmds & Self::CMD_ROT_UP    != 0 { self.pitch += rot; }
        if cmds & Self::CMD_ROT_DOWN  != 0 { self.pitch -= rot; }
        if self.pitch > cgmath::Deg(89.0) { self.pitch = cgmath::Deg(89.0); }
        if self.pitch < cgmath::Deg(-89.0) { self.pitch = cgmath::Deg(-89.0); }

        let front = cgmath::Vector3::new(self.yaw.cos(), 0.0, self.yaw.sin());
        let up = cgmath::Vector3::unit_y();

        if cmds & Self::CMD_FRONT != 0 { self.pos += mov * front; }
        if cmds & Self::CMD_BACK  != 0 { self.pos -= mov * front; }
        if cmds & Self::CMD_RIGHT != 0 { self.pos += mov * front.cross(up); }
        if cmds & Self::CMD_LEFT  != 0 { self.pos -= mov * front.cross(up); }
        if cmds & Self::CMD_UP    != 0 { self.pos += mov * up; }
        if cmds & Self::CMD_DOWN  != 0 { self.pos -= mov * up; }
    }

    pub fn get_view_proj_mat(&self) -> cgmath::Matrix4<f32> {
        let pitch_cos = self.pitch.cos();
        let facing = cgmath::Vector3::new(
            self.yaw.cos() * pitch_cos,
            self.pitch.sin(),
            self.yaw.sin() * pitch_cos
        );

        let view_mat = cgmath::Matrix4::look_to_rh(self.pos, facing, cgmath::Vector3::unit_y());

        let proj_mat = cgmath::perspective(self.fovy, self.aspect, self.near, self.far);

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

pub struct CameraControl {
    cmds: u32,
    updated_at: instant::Instant,
    rot_right: f32,
    rot_up: f32
}

impl CameraControl {
    pub fn new() -> Self {
        Self {
            cmds: 0,
            updated_at: instant::Instant::now(),
            rot_right: 0.0,
            rot_up: 0.0
        }
    }

    pub fn handle_events(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                input: KeyboardInput {
                    state,
                    virtual_keycode: Some(keycode),
                    ..
                },
                ..
            } => {
                let mut cmds = 0u32;
                let ok = match keycode {
                    VirtualKeyCode::Right  => { cmds |= Camera::CMD_ROT_RIGHT; true }
                    VirtualKeyCode::Left   => { cmds |= Camera::CMD_ROT_LEFT ; true }
                    VirtualKeyCode::Up     => { cmds |= Camera::CMD_ROT_UP   ; true }
                    VirtualKeyCode::Down   => { cmds |= Camera::CMD_ROT_DOWN ; true }
                    VirtualKeyCode::W      => { cmds |= Camera::CMD_FRONT    ; true }
                    VirtualKeyCode::S      => { cmds |= Camera::CMD_BACK     ; true }
                    VirtualKeyCode::D      => { cmds |= Camera::CMD_RIGHT    ; true }
                    VirtualKeyCode::A      => { cmds |= Camera::CMD_LEFT     ; true }
                    VirtualKeyCode::Space  => { cmds |= Camera::CMD_UP       ; true }
                    VirtualKeyCode::LShift => { cmds |= Camera::CMD_DOWN     ; true }
                    _ => false
                };
                if *state == ElementState::Pressed {
                    self.cmds |= cmds;
                } else {
                    self.cmds &= !cmds;
                }
                ok
            },
            _ => false
        }
    }

    pub fn handle_mouse_move(&mut self, x: f64, y: f64) {
        self.rot_right += x as f32;
        self.rot_up -= y as f32;
    }

    pub fn update_camera(&mut self, camera: &mut Camera) {
        let now = instant::Instant::now();
        let span = now - self.updated_at;
        self.updated_at = now;
        camera.handle_cmds(self.cmds, span.as_secs_f32(), self.rot_right, self.rot_up);
        self.rot_right = 0.0;
        self.rot_up = 0.0;
    }
}
