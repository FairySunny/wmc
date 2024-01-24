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

    var vertex_idx = idx;
    if vertex_idx >= 3u { vertex_idx = 6u - vertex_idx; }

    var local_pos: vec3<u32>;

    local_pos[dir_axis] = dir_offset;
    local_pos[dir_axis & 1u ^ 1u] = vertex_idx >> (dir_axis & 1u ^ 1u) & 1u;
    local_pos[dir_axis & 2u ^ 2u] = vertex_idx >> (dir_axis & 1u) & 1u ^ dir_offset ^ dir_axis >> 1u;

    let pos = vec3<f32>(instance.coords) + vec3<f32>(local_pos);

    out.position = camera.view_proj_mat * vec4(pos, 1.0);

    out.tex_coords = vec2(
        f32((instance.tex_id & 15u) + (vertex_idx & 1u)),
        f32((instance.tex_id >> 4u) + (vertex_idx >> 1u ^ 1u))
    ) / 16.0;

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
