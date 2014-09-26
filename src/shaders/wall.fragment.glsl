#version 330 core

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

out vec3 color;

void main() {
    float u = clamp(mod(v_tile_uv.x + u_time * v_scroll_rate, v_tile_width),
                    0.5, v_tile_width - 0.5);
    float v = clamp(mod(v_tile_uv.y, 128.0), 0.5, 127.5);

    vec2 pal = texture(u_atlas, (vec2(u, v) + v_atlas_uv) / u_atlas_size).rg;
    if (pal.g > .5) {
        discard;
    } else {
        float brightness = clamp(v_brightness - clamp((v_dist-1)/30, 0.0, 1.0),
                                 0.0001, 1.0);
        color = texture(u_palette, vec2(pal.r, 1.0 - brightness)).rgb;
    }
}
