#version 330 core

uniform mat4 u_transform;
uniform float u_time;
uniform vec2 u_atlas_size;

layout(location = 0) in vec3 a_pos;
layout(location = 1) in vec2 a_tile_uv;
layout(location = 2) in vec2 a_atlas_uv;
layout(location = 3) in float a_tile_width;
layout(location = 4) in float a_brightness;
layout(location = 5) in float a_scroll_rate;
layout(location = 6) in int a_num_frames;
layout(location = 7) in int a_frame_offset;

out float v_dist;
out vec2 v_tile_uv;
flat out vec2 v_atlas_uv;
flat out float v_tile_width;
flat out float v_brightness;

const float TICK_RATE = 8.0 / 35.0;
const float TILE_HEIGHT = 128.0;

void main() {
    v_tile_uv = a_tile_uv + vec2(u_time * a_scroll_rate, 0.0);
    if (a_num_frames == 1) {
      v_atlas_uv = a_atlas_uv;
    } else {
        float frame_index = floor(mod(
              u_time / TICK_RATE + a_frame_offset, a_num_frames));
        float atlas_u = a_atlas_uv.x + frame_index * a_tile_width;
        float atlas_v =
            a_atlas_uv.y + floor(atlas_u / u_atlas_size.x) * TILE_HEIGHT;
        v_atlas_uv = vec2(atlas_u, atlas_v);
    }
    v_tile_width = a_tile_width;
    v_brightness = a_brightness;
    vec4 projected_pos = u_transform * vec4(a_pos, 1);
    v_dist = projected_pos.w;
    gl_Position = projected_pos;
}
