#![cfg_attr(feature = "cargo-clippy", allow(clippy::forget_copy))]

use bytemuck::{offset_of, Pod, Zeroable};
use engine::ShaderVertex;
use glium::implement_vertex;
use std::sync::OnceLock;
use wgpu::VertexAttribute;

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable, Default)]
pub struct StaticVertex {
    pub a_pos: [f32; 3],
    pub _padding_1: f32,
    pub a_atlas_uv: [f32; 2],
    pub a_tile_uv: [f32; 2],
    pub a_tile_size: [f32; 2],
    pub a_scroll_rate: f32,
    pub a_row_height: f32,
    pub a_num_frames: u32,
    pub a_light: u32,
}

impl ShaderVertex for StaticVertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        static ATTRIBUTES: OnceLock<Vec<wgpu::VertexAttribute>> = OnceLock::new();
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<StaticVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &ATTRIBUTES.get_or_init(|| {
                vec![
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x3,
                        offset: offset_of!(StaticVertex, a_pos) as u64,
                        shader_location: 0,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32,
                        offset: offset_of!(StaticVertex, a_atlas_uv) as u64,
                        shader_location: 1,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x2,
                        offset: offset_of!(StaticVertex, a_tile_uv) as u64,
                        shader_location: 2,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x2,
                        offset: offset_of!(StaticVertex, a_tile_size) as u64,
                        shader_location: 3,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x2,
                        offset: offset_of!(StaticVertex, a_scroll_rate) as u64,
                        shader_location: 4,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32,
                        offset: offset_of!(StaticVertex, a_row_height) as u64,
                        shader_location: 5,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Uint32,
                        offset: offset_of!(StaticVertex, a_num_frames) as u64,
                        shader_location: 6,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Uint32,
                        offset: offset_of!(StaticVertex, a_light) as u64,
                        shader_location: 7,
                    },
                ]
            }),
        }
    }
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
#[derive(Copy, Clone, Pod, Zeroable, Default)]
pub struct SpriteVertex {
    pub a_pos: [f32; 3],
    pub _padding_1: f32,
    pub a_atlas_uv: [f32; 2],
    pub a_tile_uv: [f32; 2],
    pub a_tile_size: [f32; 2],
    pub a_local_x: f32,
    pub a_num_frames: u32,
    pub a_light: u32,
}

impl ShaderVertex for SpriteVertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        static ATTRIBUTES: OnceLock<Vec<VertexAttribute>> = OnceLock::new();
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<SpriteVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &ATTRIBUTES.get_or_init(|| {
                vec![
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x3,
                        offset: offset_of!(SpriteVertex, a_pos) as u64,
                        shader_location: 0,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x2,
                        offset: offset_of!(SpriteVertex, a_atlas_uv) as u64,
                        shader_location: 1,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x2,
                        offset: offset_of!(SpriteVertex, a_tile_uv) as u64,
                        shader_location: 2,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x2,
                        offset: offset_of!(SpriteVertex, a_tile_size) as u64,
                        shader_location: 3,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x2,
                        offset: offset_of!(SpriteVertex, a_local_x) as u64,
                        shader_location: 4,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Uint32,
                        offset: offset_of!(SpriteVertex, a_num_frames) as u64,
                        shader_location: 5,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Uint32,
                        offset: offset_of!(SpriteVertex, a_light) as u64,
                        shader_location: 6,
                    },
                ]
            }),
        }
    }
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
#[derive(Copy, Clone, Pod, Zeroable, Default)]
pub struct SkyVertex {
    pub a_pos: [f32; 3],
    pub _padding_1: f32,
}

impl ShaderVertex for SkyVertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        static ATTRIBUTES: OnceLock<Vec<VertexAttribute>> = OnceLock::new();
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<SkyVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &ATTRIBUTES.get_or_init(|| {
                vec![wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: offset_of!(SkyVertex, a_pos) as u64,
                    shader_location: 0,
                }]
            }),
        }
    }
}

implement_vertex! {
    SkyVertex,
    a_pos,
}
