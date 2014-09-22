#version 330 core

uniform mat4 u_transform;
uniform vec3 u_eye;

layout(location = 0) in vec3 a_pos;
layout(location = 1) in vec3 a_normal;

varying vec3 v_normal;
varying vec3 v_pos;

void main() {
  v_normal = a_normal;
  v_pos = a_pos;
  gl_Position = u_transform * vec4(a_pos, 1);
}
