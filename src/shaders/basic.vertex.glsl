#version 330 core

uniform mat4 mvp_transform;
uniform vec3 eye_uniform;

layout(location = 0) in vec3 pos_model;
layout(location = 1) in vec3 normal_model;

varying vec3 normal;
varying vec3 eye;
varying vec3 vpos;

void main() {
  vec4 pos = mvp_transform * vec4(pos_model, 1);

  eye = eye_uniform;
  normal = normal_model;
  vpos = pos_model;

  gl_Position = pos;
}
