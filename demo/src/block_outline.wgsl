struct CameraUniform {
    view_proj_mat: mat4x4<f32>
}

@group(0) @binding(0)
var<uniform> camera: CameraUniform;
@group(1) @binding(0)
var<uniform> position: vec3<i32>;

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

    let local_pos = vec3<u32>(xz_idx & 1u, y_idx, xz_idx >> 1u);

    let pos = vec3<f32>(position) + vec3<f32>(local_pos);

    return VertexOutput(camera.view_proj_mat * vec4(pos, 1.0));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
}
