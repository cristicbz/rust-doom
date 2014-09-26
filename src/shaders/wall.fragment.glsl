#version 330 core
out vec3 color;

uniform vec2 u_atlas_size;
uniform sampler2D u_atlas;
uniform sampler2D u_palette;
uniform float u_time;

in float v_dist;
in vec2 v_tile_uv;
flat in float v_scroll_rate;
flat in vec2 v_atlas_uv;
flat in float v_tile_width;
flat in float v_brightness;

const float BRIGHT_BIAS = 1e-4;
const float DISTANCE_FALOFF = 30.0;
const float TILE_HEIGHT = 128.0;

void main() {
    float u = clamp(mod(v_tile_uv.x + u_time * v_scroll_rate, v_tile_width),
                    0.5, v_tile_width - 0.5);
    float v = clamp(mod(v_tile_uv.y, TILE_HEIGHT), 0.5, TILE_HEIGHT - 0.5);

    vec2 palette_index =
        texture(u_atlas, (vec2(u, v) + v_atlas_uv) / u_atlas_size).rg;
    if (palette_index.g > .5) {  // Transparent pixel.
        discard;
    } else {
        float colormap_index = 1.0 -
            clamp(v_brightness + clamp((1.0 - v_dist) / DISTANCE_FALOFF,
                                       0.0, 1.0),
                  BRIGHT_BIAS, 1.0 - BRIGHT_BIAS);
        color = texture(u_palette, vec2(palette_index.r, colormap_index)).rgb;
    }
}
