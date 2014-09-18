#version 330 core
out vec3 color;

varying float w;

void main() {
  float w2 = w*w;
  color = vec3(w2*0.7, w2, w2*0.3);
}
