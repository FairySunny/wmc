struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>
}

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VertexOutput {
    var out: VertexOutput;
    let coords = vec2(f32(idx & 1u), f32(idx >> 1u)) * 2.0;
    out.position = vec4(coords.x * 2.0 - 1.0, 1.0 - coords.y * 2.0, 0.0, 1.0);
    out.tex_coords = coords;
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
