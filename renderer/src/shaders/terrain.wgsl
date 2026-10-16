struct InstanceInput {
    @location(0) position: vec3<i32>,
    @location(1) direction: u32,
    @location(2) texture_id: u32
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>
}

@group(0) @binding(0)
var<uniform> view_proj_mat: mat4x4<f32>;

@vertex
fn vs_main(instance: InstanceInput, @builtin(vertex_index) idx: u32) -> VertexOutput {
    let dir_axis = instance.direction >> 1u;
    let dir_offset = instance.direction & 1u;

    var vertex_idx = idx;
    if vertex_idx >= 3u { vertex_idx = 6u - vertex_idx; }

    var local_pos: vec3<u32>;
    local_pos[dir_axis] = dir_offset;
    local_pos[dir_axis & 1u ^ 1u] = vertex_idx >> (dir_axis & 1u ^ 1u) & 1u;
    local_pos[dir_axis & 2u ^ 2u] = vertex_idx >> (dir_axis & 1u) & 1u ^ dir_offset ^ dir_axis >> 1u;

    let pos = vec3<f32>(instance.position) + vec3<f32>(local_pos);

    var out: VertexOutput;
    out.position = view_proj_mat * vec4(pos, 1f);
    out.uv = vec2(
        f32((instance.texture_id & 15u) + (vertex_idx & 1u)),
        f32((instance.texture_id >> 4u) + (vertex_idx >> 1u ^ 1u))
    ) / 16f;
    return out;
}

@group(1) @binding(0)
var tex: texture_2d<f32>;
@group(1) @binding(1)
var tex_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(tex, tex_sampler, in.uv);
}
