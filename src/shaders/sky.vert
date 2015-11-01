uniform mat4 u_modelview;
uniform mat4 u_projection;

in vec3 a_pos;

flat out vec2 v_r;
out vec4 v_p;

void main() {
    mat4 transform = u_projection * u_modelview;
    vec4 forward = transform[2];
    v_r = vec2(atan(forward.x, forward.z), forward.y / forward.w);
    vec4 projected_pos = transform * vec4(a_pos, 1);
    v_p = projected_pos;
    gl_Position = projected_pos;
}
