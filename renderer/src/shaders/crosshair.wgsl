struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>
}

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VertexOutput {
    let pos = vec2(f32(idx << 1u & 2u), f32(idx & 2u));

    var out: VertexOutput;
    out.position = vec4(pos * 2f - 1f, 0f, 1f);
    out.uv = vec2(pos.x, 1f - pos.y);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let size = in.position.xy / in.uv.xy;

    // let len1 = size.y / 50f;
    let len1 = 20f;
    let len2 = len1 / 10f;
    let dists = abs(in.position.xy - size / 2f);

    if dists.x < len1 && dists.y < len2 || dists.y < len1 && dists.x < len2 {
        return vec4(1f, 1f, 1f, 0f);
    }
    return vec4(0f);
}
