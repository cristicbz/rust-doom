uniform mat4 u_modelview;
uniform mat4 u_projection;
uniform vec2 u_atlas_size;
uniform float u_time;
uniform float u_lights[256];

layout(location = 0) in vec3 a_pos;
layout(location = 1) in float a_local_x;
layout(location = 2) in vec2 a_atlas_uv;
layout(location = 3) in vec2 a_tile_uv;
layout(location = 4) in vec2 a_tile_size;
layout(location = 5) in int a_num_frames;
layout(location = 6) in int a_frame_offset;
layout(location = 7) in int a_light;

out float v_dist;
out vec2 v_tile_uv;
flat out vec2 v_atlas_uv;
flat out vec2 v_tile_size;
flat out float v_light;

const float ANIM_FPS = 8.0 / 35.0;

void main() {
    v_tile_uv = a_tile_uv;
    v_atlas_uv = a_atlas_uv;
    v_tile_size = a_tile_size;
    v_light = u_lights[a_light];

    vec3 right = vec3(u_modelview[0][0], u_modelview[1][0], u_modelview[2][0]);
    vec3 pos = a_pos + right * a_local_x;
    vec4 projected_pos = u_projection * (u_modelview * vec4(pos, 1.0));

    v_dist = projected_pos.w;
    gl_Position = projected_pos;
}
