@group(0) @binding(0) var<uniform> u_projection: mat4x4<f32>;
@group(0) @binding(1) var u_sampler: sampler;
@group(0) @binding(2) var<uniform> u_lights: array<u8>;
@group(0) @binding(3) var<uniform> u_atlas_size: vec2<f32>;
@group(0) @binding(4) var<uniform> u_time: f32;

@group(1) @binding(0) var u_atlas: texture_2d<f32>;
@group(1) @binding(1) var u_palette: texture_2d<f32>;

@group(1) @binding(0) var u_atlas: sampler;
@group(1) @binding(1) var u_palette: sampler;

@group(2) @binding(0) var<uniform> u_modelview: mat4x4<f32>;

struct VertexInput {
    @location(0) a_pos: vec3<f32>;
    @location(1) a_atlas_uv: vec2<f32>;
    @location(2) a_tile_uv: vec2<f32>;
    @location(3) a_tile_size: vec2<f32>;
    @location(4) a_local_x: f32;
    @location(5) a_num_frames: i32;
    @location(6) a_light: i32;
}

struct VertexOutput {
    @builtin(position) clip_position: vec3<f32>;
    @location(0) v_dist: f32;
    @location(1) v_tile_uv: vec2<f32>;
    @location(2) v_atlas_uv: vec2<f32>;
    @location(3) v_tile_size: vec2<f32>;
    @location(4) v_light: f32;
}

const ANIM_FPS: f32 = 8.0 / 35.0;

@vertex
fn main_vs(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    v_tile_uv = in.a_tile_uv;
    if in.a_num_frames == 1 {
        out.v_atlas_uv = in.a_atlas_uv;
    } else {
        let frame_index = u_time / ANIM_FPS;
        frame_index = floor(mod(frame_index, f32(in.a_num_frames)));

        let atlas_u = in.a_atlas_uv.x + frame_index * in.a_tile_size.x;
        let n_rows_down = ceil((atlas_u + in.a_tile_size.x) / u_atlas_size.x) - 1.0;
        atlas_u += mod(u_atlas_size.x - in.a_atlas_uv.x, in.a_tile_size.x) * n_rows_down;

        let atlas_v = in.a_atlas_uv.y + n_rows_down * in.a_tile_size.y;
        out.v_atlas_uv = vec2(atlas_u, atlas_v);
    }
    out.v_tile_size = in.a_tile_size;

    vec3right = vec3(u_modelview[0][0], u_modelview[1][0], u_modelview[2][0]);
    vec3pos = in.a_pos + right * in.a_local_x;
    vec4projected_pos = u_projection * (u_modelview * vec4(pos, 1.0));
    out.v_light = u_lights[in.a_light];
    out.v_dist = projected_pos.w;
    out.clip_position = projected_pos;
    return out;
}

const DIST_SCALE: f32 = 1.0;
const LIGHT_SCALE: f32 = 2.0;

@fragment
fn main_fs(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = mod(in.v_tile_uv, in.v_tile_size) + in.v_atlas_uv;
    let palette_index = textureSample(u_atlas, u_sampler, uv / u_atlas_size).rg;
    if palette_index.g > .5 {  // Transparent pixel.
        discard;
    } else {
        float dist_term = min(1.0, 1.0 - DIST_SCALE / (in.v_dist + DIST_SCALE));
        float light = min(in.v_light, in.v_light * LIGHT_SCALE - dist_term);
        return vec4(textureSample(u_palette, u_sampler, vec2(palette_index.r, 1.0 - light)).rgb, 1.0);
    }
}
