#version 300 es
precision mediump float;

out vec3 color;

uniform sampler2D u_palette;
uniform sampler2D u_atlas;
uniform vec2 u_atlas_size;

in vec3 v_pos;
flat in vec2 v_offset;
flat in float v_light;
in float v_dist;

const float WORLD_TO_PIXEL = 100.0;
const float TILE_SIZE = 64.0;
const float LIGHT_SCALE = 1.75;
const float LIGHT_BIAS = 1e-5;

void main() {
    vec2 uv = mod(-v_pos.xz * WORLD_TO_PIXEL, TILE_SIZE) + v_offset;
    float palette_index = texture(u_atlas, uv / u_atlas_size).r;
    float dist_term = min(1.0, 1.0 - 1.2 / (v_dist + 1.2));
    float light = min(v_light, v_light * LIGHT_SCALE - dist_term);

    light = clamp(light, LIGHT_BIAS, 1.0 - LIGHT_BIAS);

    // Palettized lighting:
    //color = texture(u_palette, vec2(palette_index, 1.0 - light)).rgb;

    // Linear interpolated lighting:
    color = texture(u_palette, vec2(palette_index, 0.0)).rgb * vec3(light, light, light);
}
