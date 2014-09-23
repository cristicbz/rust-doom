#version 330 core

uniform mat4 u_transform;

layout(location = 0) in vec3 a_pos;
layout(location = 1) in vec2 a_tile_uv;
layout(location = 2) in vec2 a_atlas_uv;
layout(location = 3) in float a_tile_width;
layout(location = 4) in float a_brightness;

out float v_dist;
out vec2 v_tile_uv;
flat out vec2 v_atlas_uv;
flat out float v_tile_width;
flat out float v_brightness;

void main() {
    v_tile_uv = a_tile_uv;
    v_atlas_uv = a_atlas_uv;
    v_tile_width = a_tile_width;
    v_brightness = a_brightness;
    vec4 projected_pos = u_transform * vec4(a_pos, 1); 
    v_dist = projected_pos.w;
    gl_Position = projected_pos;
}
