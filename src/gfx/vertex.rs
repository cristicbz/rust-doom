use Bounds;
use error::{NeededBy, Result};
use glium::VertexBuffer;
use math::{Vec2f, Vec3f};
use Window;

pub type SkyBuffer = VertexBuffer<SkyVertex>;
pub type SpriteBuffer = VertexBuffer<SpriteVertex>;
pub type StaticBuffer = VertexBuffer<StaticVertex>;


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

implement_vertex!(StaticVertex,
                  a_pos, a_atlas_uv, a_tile_uv, a_tile_size, a_scroll_rate, a_row_height,
                  a_num_frames, a_light);

pub struct FlatBufferBuilder(Vec<StaticVertex>);

impl FlatBufferBuilder {
    pub fn new() -> Self {
        FlatBufferBuilder(Vec::with_capacity(256))
    }

    pub fn push(&mut self, xz: &Vec2f, y: f32, light_info: u8, bounds: &Bounds) -> &mut Self {
        self.0.push(StaticVertex {
            a_pos: [xz[0], y, xz[1]],
            a_atlas_uv: [bounds.pos[0], bounds.pos[1]],
            a_tile_uv: [-xz[0] * 100.0, -xz[1] * 100.0],
            a_tile_size: [bounds.size[0], bounds.size[1]],
            a_scroll_rate: 0.0,
            a_num_frames: bounds.num_frames as u8,
            a_row_height: bounds.row_height as f32,
            a_light: light_info,
        });
        self
    }

    pub fn build(&self, window: &Window) -> Result<StaticBuffer> {
        VertexBuffer::immutable(window.facade(), &self.0).needed_by("flats vertex buffer")
    }
}

pub struct WallBufferBuilder(Vec<StaticVertex>);

impl WallBufferBuilder {
    pub fn new() -> Self {
        WallBufferBuilder(Vec::with_capacity(256))
    }

    pub fn push(&mut self,
                xz: &Vec2f,
                y: f32,
                tile_u: f32,
                tile_v: f32,
                light_info: u8,
                scroll_rate: f32,
                bounds: &Bounds)
                -> &mut Self {
        self.0.push(StaticVertex {
            a_pos: [xz[0], y, xz[1]],
            a_atlas_uv: [bounds.pos[0], bounds.pos[1]],
            a_tile_uv: [tile_u, tile_v],
            a_tile_size: [bounds.size[0], bounds.size[1]],
            a_scroll_rate: scroll_rate,
            a_num_frames: bounds.num_frames as u8,
            a_row_height: bounds.row_height as f32,
            a_light: light_info,
        });
        self
    }

    pub fn build(&self, window: &Window) -> Result<StaticBuffer> {
        VertexBuffer::immutable(window.facade(), &self.0).needed_by("walls vertex buffer")
    }
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

implement_vertex!(SpriteVertex,
                  a_pos, a_atlas_uv, a_tile_uv, a_tile_size, a_local_x, a_num_frames, a_light);

pub struct DecorBufferBuilder(Vec<SpriteVertex>);

impl DecorBufferBuilder {
    pub fn new() -> Self {
        DecorBufferBuilder(Vec::with_capacity(256))
    }

    pub fn push(&mut self,
                pos: &Vec3f,
                local_x: f32,
                tile_u: f32,
                tile_v: f32,
                bounds: &Bounds,
                light_info: u8)
                -> &mut Self {
        self.0.push(SpriteVertex {
            a_pos: [pos[0], pos[1], pos[2]],
            a_local_x: local_x,
            a_atlas_uv: [bounds.pos[0], bounds.pos[1]],
            a_tile_uv: [tile_u, tile_v],
            a_tile_size: [bounds.size[0], bounds.size[1]],
            a_num_frames: 1,
            a_light: light_info,
        });
        self
    }

    pub fn build(&self, window: &Window) -> Result<SpriteBuffer> {
        VertexBuffer::immutable(window.facade(), &self.0).needed_by("decors buffer")
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct SkyVertex {
    pub a_pos: [f32; 3],
}

implement_vertex!(SkyVertex, a_pos);

pub struct SkyBufferBuilder(Vec<SkyVertex>);

impl SkyBufferBuilder {
    pub fn new() -> Self {
        SkyBufferBuilder(Vec::with_capacity(256))
    }

    pub fn push(&mut self, xz: &Vec2f, y: f32) -> &mut Self {
        self.0.push(SkyVertex { a_pos: [xz[0], y, xz[1]] });
        self
    }

    pub fn build(&self, window: &Window) -> Result<SkyBuffer> {
        VertexBuffer::immutable(window.facade(), &self.0).needed_by("sky vertex buffer")
    }
}
