#version 300 es
precision mediump float;

out vec3 color;

uniform vec2 u_atlas_size;
uniform sampler2D u_atlas;
uniform sampler2D u_palette;

in float v_dist;
in vec2 v_tile_uv;
flat in vec2 v_atlas_uv;
flat in float v_tile_width;
flat in float v_light;

const float TILE_HEIGHT = 128.0;
const float LIGHT_SCALE = 1.75;
const float LIGHT_BIAS = 1e-4;

void main() {
    vec2 uv = mod(v_tile_uv, vec2(v_tile_width, TILE_HEIGHT)) + v_atlas_uv;
    vec2 palette_index = texture(u_atlas, uv / u_atlas_size).rg;
    if (palette_index.g > .5) {  // Transparent pixel.
        discard;
    } else {
        float dist_term = min(1.0, 1.0 - 1.2 / (v_dist + 1.2));
        float light = min(v_light, v_light * LIGHT_SCALE - dist_term);

        light = clamp(light, LIGHT_BIAS, 1.0 - LIGHT_BIAS);

        // Palettized lighting:
        //color = texture(u_palette, vec2(palette_index.r, 1.0 - light)).rgb;

        // Linear interpolated lighting:
        color = texture(u_palette, vec2(palette_index.r, 0.0)).rgb * vec3(light, light, light);
    }
}
