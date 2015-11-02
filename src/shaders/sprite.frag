precision mediump float;

out vec3 color;

uniform vec2 u_atlas_size;
uniform sampler2D u_atlas;
uniform sampler2D u_palette;

in float v_dist;
in vec2 v_tile_uv;
flat in vec2 v_atlas_uv;
flat in vec2 v_tile_size;
flat in float v_light;

const float DIST_SCALE = 1.0;
const float LIGHT_SCALE = 2.0;

void main() {
    vec2 uv = mod(v_tile_uv, v_tile_size) + v_atlas_uv;
    vec2 palette_index = texture(u_atlas, uv / u_atlas_size).rg;
    if (palette_index.g > .5) {  // Transparent pixel.
        discard;
    } else {
        float dist_term = min(1.0, 1.0 - DIST_SCALE / (v_dist + DIST_SCALE));
        float light = min(v_light, v_light * LIGHT_SCALE - dist_term);
        color = texture(u_palette, vec2(palette_index.r, 1.0 - light)).rgb;
    }
}
