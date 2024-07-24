use winit::event::{WindowEvent, KeyboardInput, VirtualKeyCode, ElementState};

pub struct CameraControl {
    cmds: u32,
    updated_at: instant::Instant,
    rot_right: f32,
    rot_up: f32,
    mouse_rot: bool
}

impl CameraControl {
    const CMD_FRONT    : u32 = 1 << 0;
    const CMD_BACK     : u32 = 1 << 1;
    const CMD_RIGHT    : u32 = 1 << 2;
    const CMD_LEFT     : u32 = 1 << 3;
    const CMD_UP       : u32 = 1 << 4;
    const CMD_DOWN     : u32 = 1 << 5;
    const CMD_ROT_RIGHT: u32 = 1 << 6;
    const CMD_ROT_LEFT : u32 = 1 << 7;
    const CMD_ROT_UP   : u32 = 1 << 8;
    const CMD_ROT_DOWN : u32 = 1 << 9;
    const MOV_SPEED: f32 = 5.0;
    const ROT_SPEED: cgmath::Deg<f32> = cgmath::Deg(0.2);

    pub fn new() -> Self {
        Self {
            cmds: 0,
            updated_at: instant::Instant::now(),
            rot_right: 0.0,
            rot_up: 0.0,
            mouse_rot: true
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
                    VirtualKeyCode::Right  => { cmds |= Self::CMD_ROT_RIGHT; true }
                    VirtualKeyCode::Left   => { cmds |= Self::CMD_ROT_LEFT ; true }
                    VirtualKeyCode::Up     => { cmds |= Self::CMD_ROT_UP   ; true }
                    VirtualKeyCode::Down   => { cmds |= Self::CMD_ROT_DOWN ; true }
                    VirtualKeyCode::W      => { cmds |= Self::CMD_FRONT    ; true }
                    VirtualKeyCode::S      => { cmds |= Self::CMD_BACK     ; true }
                    VirtualKeyCode::D      => { cmds |= Self::CMD_RIGHT    ; true }
                    VirtualKeyCode::A      => { cmds |= Self::CMD_LEFT     ; true }
                    VirtualKeyCode::Space  => { cmds |= Self::CMD_UP       ; true }
                    VirtualKeyCode::LShift => { cmds |= Self::CMD_DOWN     ; true }
                    VirtualKeyCode::E if *state == ElementState::Pressed => {
                        self.mouse_rot = !self.mouse_rot;
                        true
                    }
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
        if self.mouse_rot {
            self.rot_right += x as f32;
            self.rot_up -= y as f32;
        }
    }

    pub fn update_camera(&mut self, camera: &mut renderer::camera::Camera) {
        use cgmath::Angle;

        let now = instant::Instant::now();
        let time_span = (now - self.updated_at).as_secs_f32();
        self.updated_at = now;

        let mov = Self::MOV_SPEED * time_span;
        let rot = Self::ROT_SPEED * 500.0 * time_span;

        camera.yaw += Self::ROT_SPEED * self.rot_right;
        camera.pitch += Self::ROT_SPEED * self.rot_up;
        if self.cmds & Self::CMD_ROT_RIGHT != 0 { camera.yaw += rot; }
        if self.cmds & Self::CMD_ROT_LEFT  != 0 { camera.yaw -= rot; }
        if self.cmds & Self::CMD_ROT_UP    != 0 { camera.pitch += rot; }
        if self.cmds & Self::CMD_ROT_DOWN  != 0 { camera.pitch -= rot; }
        if camera.pitch > cgmath::Deg(89.0) { camera.pitch = cgmath::Deg(89.0); }
        if camera.pitch < cgmath::Deg(-89.0) { camera.pitch = cgmath::Deg(-89.0); }

        let front = cgmath::Vector3::new(camera.yaw.cos(), 0.0, camera.yaw.sin());
        let up = cgmath::Vector3::unit_y();

        if self.cmds & Self::CMD_FRONT != 0 { camera.pos += mov * front; }
        if self.cmds & Self::CMD_BACK  != 0 { camera.pos -= mov * front; }
        if self.cmds & Self::CMD_RIGHT != 0 { camera.pos += mov * front.cross(up); }
        if self.cmds & Self::CMD_LEFT  != 0 { camera.pos -= mov * front.cross(up); }
        if self.cmds & Self::CMD_UP    != 0 { camera.pos += mov * up; }
        if self.cmds & Self::CMD_DOWN  != 0 { camera.pos -= mov * up; }

        self.rot_right = 0.0;
        self.rot_up = 0.0;
    }
}
