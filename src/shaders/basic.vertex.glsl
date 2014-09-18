#version 330 core

uniform mat4 mvp_transform;

layout(location = 0) in vec3 pos_model;

varying float w;

void main() {
  vec4 pos = mvp_transform * vec4(pos_model, 1);
  w = max(1.0 - pos.w / 30.0, 0);

  gl_Position = pos;
}
