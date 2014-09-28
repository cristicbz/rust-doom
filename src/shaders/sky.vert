#version 330 core
uniform mat4 u_transform;

layout(location = 0) in vec3 a_pos;

flat out vec2 v_r;
out vec4 v_p;

void main() {
    vec4 forward = u_transform[2];
    v_r = vec2(atan(forward.x, forward.z), forward.y / forward.w);
    vec4 projected_pos = u_transform * vec4(a_pos, 1);
    v_p = projected_pos;
    gl_Position = projected_pos;
}
