struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) coords: vec2<f32>
}

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VertexOutput {
    var out: VertexOutput;

    var vertex_idx = idx;
    if vertex_idx >= 3u { vertex_idx = 6u - vertex_idx; }

    let pos = vec2(
        f32(vertex_idx & 1u),
        f32(vertex_idx >> 1u)
    );

    out.position = vec4(pos.x * 2.0 - 1.0, pos.y * 2.0 - 1.0, 0.0, 1.0);

    out.coords = vec2(pos.x, 1.0 - pos.y);

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let size = in.position.xy / in.coords.xy;

    let dists = abs(in.position.xy - size / 2.0);

    // let len1 = size.y / 50.0;
    let len1 = 20.0;
    let len2 = len1 / 10.0;

    if dists.x < len1 && dists.y < len2 || dists.y < len1 && dists.x < len2 {
        return vec4(1.0, 1.0, 1.0, 0.0);
    }

    return vec4(0.0);
}
