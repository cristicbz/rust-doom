#version 330 core

uniform mat4 u_transform;

layout(location = 0) in vec3 a_pos;
layout(location = 1) in vec2 a_offset;
layout(location = 2) in float a_brightness;

flat out vec2 v_offset;
flat out float v_brightness;
out float v_dist;
out vec3 v_pos;

void main() {
  v_pos = a_pos;
  v_offset = a_offset;
  v_brightness = a_brightness;
  vec4 projected_pos = u_transform * vec4(a_pos, 1);
  v_dist = projected_pos.w;
  gl_Position = projected_pos;
}
