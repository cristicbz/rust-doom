@group(0) @binding(0) var u_tex: texture_2d<f32>;
@group(0) @binding(1) var u_sampler: sampler;

struct VertexInput {
    @location(0) a_pos: vec2<f32>,
    @location(1) a_uv: vec2<f32>,
}

struct VertexOutput {
    @location(0) v_uv: vec2<f32>,
    @builtin(position) clip_position: vec4<f32>,
}

@vertex
fn main_vs(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.v_uv = in.a_uv;
    out.clip_position = vec4(in.a_pos, 0.0, 1.0);
    return out;
}

@fragment
fn main_fs(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_color = textureSample(u_tex, u_sampler, in.v_uv);
    return vec4(tex_color.g, tex_color.g, tex_color.g, tex_color.r);
}
