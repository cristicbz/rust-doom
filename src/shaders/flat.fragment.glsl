#version 330 core
out vec3 color;

uniform sampler2D u_palette;
uniform sampler2D u_texture;

in vec3 v_pos;
flat in vec2 v_offset;
flat in float v_brightness;
in float v_dist;

void main() {
  vec2 tile_size = 64.0 / vec2(textureSize(u_texture, 0));
  vec2 uv = mod(v_pos.xz * vec2(.5, 1.0) / (.64 * 4), tile_size);
  float pal = texture2D(u_texture, uv + v_offset).r;
  float brightness = clamp(v_brightness - clamp((v_dist - 1.0)/30, 0.0, 1.0),
                           0.0001, 1.0);
  color = texture2D(u_palette, vec2(pal, 1.0 - brightness)).rgb;
}
