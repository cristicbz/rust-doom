@group(0) @binding(0) var<uniform> u_viewproj: mat4x4<f32>;
// TODO: There should be a separate sampler for the palette, using clamp semantics rather than repeat
@group(0) @binding(1) var u_sampler: sampler;
@group(0) @binding(4) var u_palette: texture_2d<f32>;

@group(1) @binding(0) var u_texture: texture_2d<f32>;
@group(1) @binding(2) var<uniform> u_tiled_band_size: f32;

@group(2) @binding(0) var<uniform> u_model: mat4x4<f32>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) v_r: vec2<f32>,
    @location(1) v_p: vec4<f32>,
}

@vertex
fn main_vs(@location(0) a_pos: vec3<f32>) -> VertexOutput {
    var out: VertexOutput;
    let transform = u_viewproj * u_model;
    let forward = transform[2];
    out.v_r = vec2(atan(forward.x / forward.z), forward.y / forward.w);
    let projected_pos = transform * vec4(a_pos, 1);
    out.v_p = projected_pos;
    out.clip_position = projected_pos;
    return out;
}

@fragment
fn main_fs(in: VertexOutput) -> @location(0) vec4<f32> {
    var uv = vec2(in.v_p.x, in.v_p.y) / in.v_p.w * vec2(1, -1);
    uv = vec2(uv.x - 4.0 * in.v_r.x / 3.14159265358, uv.y + 1.0 + in.v_r.y);
    if uv.y < 0.0 {
        uv.y = abs((-uv.y + u_tiled_band_size) % (u_tiled_band_size * 2.0) - u_tiled_band_size);
    } else if uv.y >= 2.0 {
        uv.y = abs((uv.y - 2.0 + u_tiled_band_size) % (u_tiled_band_size * 2.0) - u_tiled_band_size);
    } else if uv.y >= 1.0 {
        uv.y = 1.0 - uv.y;
    }
    let palette_index = textureSample(u_texture, u_sampler, uv).r;
    return vec4(textureSample(u_palette, u_sampler, vec2(palette_index, 0.0)).rgb, 1.0);
}
