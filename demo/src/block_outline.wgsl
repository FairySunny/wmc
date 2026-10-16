@group(0) @binding(0)
var<uniform> position: vec4<i32>; // xyz with padding
@group(1) @binding(0)
var<uniform> view_proj_mat: mat4x4<f32>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>
}

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VertexOutput {
    let idx_in_line = idx & 1u;
    var xz_idx = idx >> 1u & 3u;
    var y_idx = idx >> 3u;

    if y_idx == 0u || y_idx == 1u {
        xz_idx = xz_idx + idx_in_line & 3u;
        if xz_idx >= 2u { xz_idx = 5u - xz_idx; }
    } else {
        y_idx = idx_in_line;
    }

    let local_pos = vec3(xz_idx & 1u, y_idx, xz_idx >> 1u);

    let pos = vec3<f32>(position.xyz) + vec3<f32>(local_pos);

    return VertexOutput(view_proj_mat * vec4(pos, 1f));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4(0f, 0f, 0f, 1f);
}
