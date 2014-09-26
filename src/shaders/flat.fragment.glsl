#version 330 core
out vec3 color;

uniform sampler2D u_palette;
uniform sampler2D u_atlas;
uniform vec2 u_atlas_size;

in vec3 v_pos;
flat in vec2 v_offset;
flat in float v_brightness;
in float v_dist;

const float WORLD_TO_PIXEL = 100.0;
const float TILE_SIZE = 64.0;
const float BRIGHT_BIAS = 1e-4;
const float DISTANCE_FALOFF = 30.0;

void main() {
    vec2 uv = mod(v_pos.xz * WORLD_TO_PIXEL, TILE_SIZE);
    uv = vec2(clamp(uv.x, 0.0, TILE_SIZE - 1.0),
              clamp(uv.y, 0.0, TILE_SIZE - 1.0));
    uv += v_offset;
    uv /= u_atlas_size;
    float palette_index = texture2D(u_atlas, uv).r;
    float colormap_index = 1.0 - clamp(
            v_brightness - clamp((v_dist - 1.0) / DISTANCE_FALOFF, 0.0, 1.0),
            BRIGHT_BIAS, 1.0 - BRIGHT_BIAS);
    color = texture2D(u_palette, vec2(palette_index, colormap_index)).rgb;
}
