#version 330 core

uniform mat4 u_transform;
uniform float u_time;
uniform vec2 u_atlas_size;

layout(location = 0) in vec3 a_pos;
layout(location = 1) in vec2 a_offset;
layout(location = 2) in float a_brightness;
layout(location = 3) in int a_num_frames;
layout(location = 4) in int a_frame_offset;

flat out vec2 v_offset;
flat out float v_brightness;
out float v_dist;
out vec3 v_pos;

const float TICK_RATE = 8.0 / 35.0;
const float TILE_SIZE = 64.0;

void main() {
  v_pos = a_pos;
  if (a_num_frames == 1) {
      v_offset = a_offset;
  } else {
      float frame_index = floor(mod(
              u_time / TICK_RATE + a_frame_offset, a_num_frames));
      float atlas_u = a_offset.x + frame_index * TILE_SIZE;
      float atlas_v = a_offset.y + floor(atlas_u / u_atlas_size.x) * TILE_SIZE;
      v_offset = vec2(atlas_u, atlas_v);
  }
  v_brightness = a_brightness * 2.0;
  vec4 projected_pos = u_transform * vec4(a_pos, 1);
  v_dist = projected_pos.w;
  gl_Position = projected_pos;
}
