#![cfg_attr(feature = "cargo-clippy", allow(clippy::forget_copy))]

use glium::implement_vertex;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct StaticVertex {
    pub a_pos: [f32; 3],
    pub a_atlas_uv: [f32; 2],
    pub a_tile_uv: [f32; 2],
    pub a_tile_size: [f32; 2],
    pub a_scroll_rate: f32,
    pub a_row_height: f32,
    pub a_num_frames: u8,
    pub a_light: u8,
}

implement_vertex! {
    StaticVertex,
    a_pos,
    a_atlas_uv,
    a_tile_uv,
    a_tile_size,
    a_scroll_rate,
    a_row_height,
    a_num_frames,
    a_light,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct SpriteVertex {
    pub a_pos: [f32; 3],
    pub a_atlas_uv: [f32; 2],
    pub a_tile_uv: [f32; 2],
    pub a_tile_size: [f32; 2],
    pub a_local_x: f32,
    pub a_num_frames: u8,
    pub a_light: u8,
}

implement_vertex! {
    SpriteVertex,
    a_pos,
    a_atlas_uv,
    a_tile_uv,
    a_tile_size,
    a_local_x,
    a_num_frames,
    a_light,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct SkyVertex {
    pub a_pos: [f32; 3],
}

implement_vertex! {
    SkyVertex,
    a_pos,
}
