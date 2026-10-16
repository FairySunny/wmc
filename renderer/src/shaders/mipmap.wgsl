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

@group(0) @binding(0)
var tex: texture_2d<f32>;
@group(0) @binding(1)
var tex_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(tex, tex_sampler, in.uv);
}
