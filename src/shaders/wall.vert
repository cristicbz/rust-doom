#version 330 core

uniform mat4 u_transform;
uniform float u_time;
uniform vec2 u_atlas_size;

layout(location = 0) in vec3 a_pos;
layout(location = 1) in vec2 a_tile_uv;
layout(location = 2) in vec2 a_atlas_uv;
layout(location = 3) in float a_tile_width;
layout(location = 4) in float a_scroll_rate;
layout(location = 5) in int a_num_frames;
layout(location = 6) in int a_frame_offset;
layout(location = 7) in int a_light;

//////////////////
// a_light uses bit-fields to encode:
// 00000   00000   000   000
// ^^^^^   ^^^^^   ^     ^^^
// LEVEL0  LEVEL1  TYPE  SYNC

const int LIGHT_FLASH = 0;
const int LIGHT_SLOW_STROBE = 1;
const int LIGHT_FLICKER = 2;
const int LIGHT_FAST_STROBE = 3;
const int LIGHT_GLOW = 4;

float noise(float x, float y) {
    return fract((1.0 + sin(x * 12.9898 + y * 78.233)) * 43758.5453);
}

float light_level() {
    float level0 = float(a_light >> 11);
    float level1 = float((a_light >> 6) & 31);
    if (level0 == level1) { return level0; }

    int type = (a_light >> 3 & 7);
    float sync = float(a_light & 15) / 15.0;

    if (type == LIGHT_GLOW) {
        float d = level0 - level1;
        float time = (u_time + sync * 3.5435) * 16 / d;
        return abs(fract(time) - 0.5) * 2.0 * d + level1;
    } else {
        float subtype = type >> 1;
        if ((type & 1) == 0) {   // Random based effect (FLASH / FLICKER)
            float time = floor(u_time * (8.0 + 12.0 * (1.0 - subtype)));
            float noise = noise(time / 1000.0 + sync, sync);
            bool pick = noise <= (subtype * 0.44 + 0.06);
            return pick ? level1 : level0;
        } else {  // Periodic strobe (SLOW / FAST).
            float time = u_time * (1.0 + subtype);
            bool pick = fract(time + sync * 3.5453) > (0.85 - subtype * .15);
            return pick ? level0 : level1;
        }
    }
}
//////////////////

out float v_dist;
out vec2 v_tile_uv;
flat out vec2 v_atlas_uv;
flat out float v_tile_width;
flat out float v_light;

const float ANIM_FPS = 8.0 / 35.0;
const float TILE_HEIGHT = 128.0;

void main() {
    v_tile_uv = a_tile_uv + vec2(u_time * a_scroll_rate, 0.0);
    if (a_num_frames == 1) {
      v_atlas_uv = a_atlas_uv;
    } else {
        float frame_index = floor(mod(
              u_time / ANIM_FPS + a_frame_offset, a_num_frames));
        float atlas_u = a_atlas_uv.x + frame_index * a_tile_width;
        float atlas_v =
            a_atlas_uv.y + floor(atlas_u / u_atlas_size.x) * TILE_HEIGHT;
        v_atlas_uv = vec2(atlas_u, atlas_v);
    }
    v_tile_width = a_tile_width;
    v_light = light_level() * 1.0 / 31.0;
    vec4 projected_pos = u_transform * vec4(a_pos, 1);
    v_dist = projected_pos.w;
    gl_Position = projected_pos;
}
