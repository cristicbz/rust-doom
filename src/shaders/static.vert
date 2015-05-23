
uniform mat4 u_transform;
uniform vec2 u_atlas_size;
uniform float u_time;
uniform float u_lights[256];

layout(location = 0) in vec3 a_pos;
layout(location = 1) in vec2 a_atlas_uv;
layout(location = 2) in vec2 a_tile_uv;
layout(location = 3) in vec2 a_tile_size;
layout(location = 4) in float a_scroll_rate;
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
    v_tile_uv = a_tile_uv + vec2(u_time * a_scroll_rate, 0.0);
    if (a_num_frames == 1) {
      v_atlas_uv = a_atlas_uv;
    } else {
        float frame_index = u_time / ANIM_FPS + float(a_frame_offset);
        frame_index = floor(mod(frame_index, float(a_num_frames)));

        float atlas_u = a_atlas_uv.x + frame_index * a_tile_size.x;
        float atlas_v = a_atlas_uv.y + floor(atlas_u / u_atlas_size.x) * a_tile_size.y;
        v_atlas_uv = vec2(atlas_u, atlas_v);
    }
    v_tile_size = a_tile_size;
    vec4 projected_pos = u_transform * vec4(a_pos, 1);
    v_dist = projected_pos.w;
    v_light = u_lights[a_light];
    gl_Position = projected_pos;
}
