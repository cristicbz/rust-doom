#version 330 core
out vec3 color;

uniform vec3 u_eye;

varying vec3 v_normal;
varying vec3 v_pos;

void main() {
  vec3 view = u_eye - v_pos;
  float d2 = dot(view, view);
  float d = sqrt(d2);
  view /= d;
  float w = min(2 / d2 + 0.7 / d, 1) *
            max(0.0, dot(v_normal, view)) * 0.95 + 0.05;
  color = vec3(w*0.3, w, w*0.3);
}
