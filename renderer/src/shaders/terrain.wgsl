struct CameraUniform {
    view_proj_mat: mat4x4<f32>
}

@group(1) @binding(0)
var<uniform> camera: CameraUniform;

struct InstanceInput {
    @location(0) coords: vec3<i32>,
    @location(1) direction: u32,
    @location(2) tex_id: u32
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>
}

@vertex
fn vs_main(@builtin(vertex_index) idx: u32, instance: InstanceInput) -> VertexOutput {
    var out: VertexOutput;

    let dir_axis = instance.direction >> 1u;
    let dir_offset = instance.direction & 1u;

    var pos_bits = idx;
    if dir_offset == 1u { pos_bits = 5u - pos_bits; }
    if pos_bits >= 3u { pos_bits = 6u - pos_bits; }

    out.tex_coords = vec2(
        f32((instance.tex_id & 15u) + (pos_bits & 1u)),
        f32((instance.tex_id >> 4u) + (pos_bits >> 1u & 1u))
    ) / 16.0;

    if dir_offset == 1u { pos_bits |= 4u; }

    let pos = vec3<f32>(instance.coords) + vec3(
        f32(pos_bits >> (2u + dir_axis) % 3u & 1u),
        f32(pos_bits >> (1u + dir_axis) % 3u & 1u),
        f32(pos_bits >> (0u + dir_axis) % 3u & 1u)
    );

    out.position = camera.view_proj_mat * vec4(pos, 1.0);

    return out;
}

@group(0) @binding(0)
var tex: texture_2d<f32>;
@group(0) @binding(1)
var tex_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(tex, tex_sampler, in.tex_coords);
}
