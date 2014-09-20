#version 330 core
out vec3 color;

varying float w;
varying vec3 normal;
varying vec3 eye;
varying vec3 vpos;

void main() {
  vec3 view = eye - vpos;
  float d2 = dot(view, view);
  float d = sqrt(d2);
  view /= d;
  float w = min(2 / d2 + 0.7 / d, 1) * max(0.0, dot(normal, view)) * 0.95 + 0.05;
  color = vec3(w*0.3, w, w*0.3);
}
