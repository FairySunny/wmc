struct CameraUniform {
    view_proj_mat: mat4x4<f32>
}

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct InstanceInput {
    @location(0) coord: vec3<i32>,
    @location(1) direction: u32,
    @location(2) texture: u32
}

@vertex
fn vs_main(@builtin(vertex_index) idx: u32, instance: InstanceInput) -> @builtin(position) vec4<f32> {
    let dir_axis = instance.direction >> 1u;
    let dir_offset = instance.direction & 1u;

    var pos_bits = idx;
    if dir_offset == 1u { pos_bits = 5u - pos_bits; }
    if pos_bits >= 3u { pos_bits = 6u - pos_bits; }
    if dir_offset == 1u { pos_bits |= 4u; }

    let pos = vec3<f32>(instance.coord) + vec3(
        f32(pos_bits >> (2u + dir_axis) % 3u & 1u),
        f32(pos_bits >> (1u + dir_axis) % 3u & 1u),
        f32(pos_bits >> (0u + dir_axis) % 3u & 1u)
    );

    return camera.view_proj_mat * vec4<f32>(pos, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(0.0, 1.0, 0.0, 1.0);
}
