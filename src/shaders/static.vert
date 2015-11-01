uniform samplerBuffer u_lights;
uniform mat4 u_projection;
uniform mat4 u_modelview;

uniform vec2 u_atlas_size;
uniform float u_time;

in vec3 a_pos;
in vec2 a_atlas_uv;
in vec2 a_tile_uv;
in vec2 a_tile_size;
in float a_scroll_rate;
in float a_row_height;
in int a_num_frames;
in int a_light;

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
        float frame_index = u_time / ANIM_FPS;
        frame_index = floor(mod(frame_index, float(a_num_frames)));

        float atlas_u = a_atlas_uv.x + frame_index * a_tile_size.x;
        float n_rows_down = ceil((atlas_u + a_tile_size.x) / u_atlas_size.x) - 1.0;
        atlas_u += mod(u_atlas_size.x - a_atlas_uv.x, a_tile_size.x) * n_rows_down;

        float atlas_v = a_atlas_uv.y + n_rows_down * a_row_height;
        v_atlas_uv = vec2(atlas_u, atlas_v);
    }
    v_tile_size = a_tile_size;
    vec4 projected_pos = u_projection * u_modelview * vec4(a_pos, 1);
    v_dist = projected_pos.w;
    v_light = texelFetch(u_lights, a_light).r;
    gl_Position = projected_pos;
}
