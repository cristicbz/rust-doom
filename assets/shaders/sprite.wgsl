@group(0) @binding(0) var<uniform> u_viewproj: mat4x4<f32>;
// TODO: There should be a separate sampler for the palette, using clamp semantics rather than repeat
@group(0) @binding(1) var u_sampler: sampler;
@group(0) @binding(2) var<storage, read> u_lights: array<u32>;
@group(0) @binding(3) var<uniform> u_time: f32;
@group(0) @binding(4) var u_palette: texture_2d<f32>;

@group(1) @binding(0) var u_atlas: texture_2d<f32>;
@group(1) @binding(1) var<uniform> u_atlas_size: vec2<f32>;

@group(2) @binding(0) var<uniform> u_model: mat4x4<f32>;
@group(2) @binding(1) var<uniform> u_right: vec3<f32>;

struct VertexInput {
    @location(0) a_pos: vec3<f32>,
    @location(1) a_atlas_uv: vec2<f32>,
    @location(2) a_tile_uv: vec2<f32>,
    @location(3) a_tile_size: vec2<f32>,
    @location(4) a_local_x: f32,
    @location(5) a_num_frames: i32,
    @location(6) a_light: i32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) v_dist: f32,
    @location(1) v_tile_uv: vec2<f32>,
    @location(2) v_atlas_uv: vec2<f32>,
    @location(3) v_tile_size: vec2<f32>,
    @location(4) v_light: f32,
}

const ANIM_FPS: f32 = 8.0 / 35.0;

@vertex
fn main_vs(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.v_tile_uv = in.a_tile_uv;
    if in.a_num_frames == 1 {
        out.v_atlas_uv = in.a_atlas_uv;
    } else {
        let frame_index = floor((u_time / ANIM_FPS) % f32(in.a_num_frames));

        var atlas_u = in.a_atlas_uv.x + frame_index * in.a_tile_size.x;
        let n_rows_down = ceil((atlas_u + in.a_tile_size.x) / u_atlas_size.x) - 1.0;
        atlas_u += (u_atlas_size.x - in.a_atlas_uv.x) % in.a_tile_size.x * n_rows_down;

        let atlas_v = in.a_atlas_uv.y + n_rows_down * in.a_tile_size.y;
        out.v_atlas_uv = vec2(atlas_u, atlas_v);
    }
    out.v_tile_size = in.a_tile_size;

    let pos = in.a_pos + u_right * in.a_local_x;
    let projected_pos = u_viewproj * (u_model * vec4(pos, 1.0));
    out.v_light = f32(u_lights[in.a_light]);
    out.v_dist = projected_pos.w;
    out.clip_position = projected_pos;
    return out;
}

const DIST_SCALE: f32 = 1.0;
const LIGHT_SCALE: f32 = 2.0;

@fragment
fn main_fs(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.v_tile_uv % in.v_tile_size + in.v_atlas_uv;
    let palette_index = textureSample(u_atlas, u_sampler, uv / u_atlas_size).rg;
    if palette_index.g > .5 {  // Transparent pixel.
        discard;
    } else {
        let dist_term = min(1.0, 1.0 - DIST_SCALE / (in.v_dist + DIST_SCALE));
        let light = min(in.v_light, in.v_light * LIGHT_SCALE - dist_term);
        return vec4(textureSample(u_palette, u_sampler, vec2(palette_index.r, 1.0 - light)).rgb, 1.0);
    }
}
