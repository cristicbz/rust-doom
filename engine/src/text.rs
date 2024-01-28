#![cfg_attr(feature = "cargo-clippy", allow(clippy::forget_copy))]

use crate::ShaderVertex;

use super::system::System;
use super::window::Window;
use bytemuck::{offset_of, Pod, Zeroable};
use failchain::{ChainErrorKind, ResultExt, UnboxedError};
use failure::Fail;
use idcontain::{Id, IdSlab};
use log::{debug, error};
use math::Pnt2f;
use rusttype::{self, Font, GlyphId, Point as FontPoint, PositionedGlyph, Scale};
use std::fs::File;
use std::io::Read;
use std::ops::{Index, IndexMut};
use std::result::Result as StdResult;
use std::str::Chars as StrChars;
use std::sync::OnceLock;
use unicode_normalization::{Recompositions, UnicodeNormalization};
use wgpu::include_wgsl;
use wgpu::util::DeviceExt;

/// A handle to a piece of text created with a `TextRenderer`.
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct TextId(Id<Text>);

/// Handles rendering of debug text to `OpenGL`.
pub struct TextRenderer {
    font: Font<'static>,
    slab: IdSlab<Text>,
    pixel_buffer: Vec<u16>,
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

#[derive(Clone, Eq, PartialEq, Debug, Fail)]
#[fail(display = "Font error: {}", 0)]
pub struct ErrorKind(String);

pub type Error = UnboxedError<ErrorKind>;
pub type Result<T> = StdResult<T, Error>;

impl ChainErrorKind for ErrorKind {
    type Error = Error;
}

impl TextRenderer {
    pub fn insert(&mut self, win: &Window, text: &str, pos: Pnt2f, padding: u32) -> TextId {
        debug!("Creating text...");
        let (width, height) = self.rasterise(text, padding);
        let texture = win.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("Text texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rg8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let sampler = win.device().create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Text sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let bind_group = win.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Text bind group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &texture.create_view(&Default::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        let (w, h) = (
            width as f32 / win.width() as f32 * 2.0,
            height as f32 / win.height() as f32 * 2.0,
        );
        let (x, y) = (pos.x * 2.0 - 1.0, 1.0 - pos.y * 2.0 - h);
        let buffer = win
            .device()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Text vertex buffer"),
                contents: bytemuck::cast_slice(&[
                    vertex(x, y, 0.0, 1.0),
                    vertex(x, y + h, 0.0, 0.0),
                    vertex(x + w, y, 1.0, 1.0),
                    vertex(x + w, y + h, 1.0, 0.0),
                ]),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let text = Text {
            buffer,
            bind_group,
            visible: true,
        };
        let id = self.slab.insert(text);
        debug!("Created text {:?}.", id);
        TextId(id)
    }

    pub fn remove(&mut self, id: TextId) -> bool {
        debug!("Removed text {:?}.", id.0);
        self.slab.remove(id.0).is_some()
    }

    pub fn text(&self, id: TextId) -> Option<&Text> {
        self.slab.get(id.0)
    }

    pub fn text_mut(&mut self, id: TextId) -> Option<&mut Text> {
        self.slab.get_mut(id.0)
    }

    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) -> Result<()> {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Text render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
                view,
            })],
            ..Default::default()
        });
        render_pass.set_pipeline(&self.pipeline);
        for text in &self.slab {
            if !text.visible {
                continue;
            }
            render_pass.set_vertex_buffer(0, text.buffer.slice(..));
            render_pass.set_bind_group(0, &text.bind_group, &[]);
            render_pass.draw(0..4, 0..1);
        }
        Ok(())
    }

    fn rasterise(&mut self, text: &str, padding: u32) -> (u32, u32) {
        debug!("Rasterising text {:?}...", text);
        let scale = Scale::uniform(POINT_SIZE);
        let (width, height) = LayoutIter::new(&self.font, scale, !0, text)
            .filter_map(|glyph| glyph.pixel_bounding_box())
            .map(|bb| (bb.max.x, bb.max.y))
            .fold((0, 0), |left, right| {
                (left.0.max(right.0), left.1.max(right.1))
            });
        let (width, height) = (width as u32 + padding * 2, height as u32 + padding * 2);
        debug!("Computed dimensions {}x{}...", width, height);

        let pixel_buffer = &mut self.pixel_buffer;
        pixel_buffer.clear();
        pixel_buffer.resize((width * height) as usize, 0x00_80);
        debug!("Resized buffer to {}...", pixel_buffer.len());
        for glyph in LayoutIter::new(&self.font, scale, width, text) {
            if let Some(bb) = glyph.pixel_bounding_box() {
                let offset_x = (bb.min.x + padding as i32) as u32;
                let offset_y = (bb.min.y + padding as i32) as u32;
                glyph.draw(|mut x, mut y, alpha| {
                    x += offset_x;
                    y += offset_y;
                    if x < width && y < height {
                        let scale = (alpha * 256.0) as u32;
                        let one_minus_scale = 256 - scale;

                        let pixel = &mut pixel_buffer[(y * width + x) as usize];

                        let new_alpha = scale * 255 / 256;

                        let red = u32::from(*pixel >> 8) * one_minus_scale + 0xff * scale;
                        let alpha = u32::from(*pixel & 0xff) * one_minus_scale + new_alpha * scale;

                        *pixel = (red | (alpha >> 8)) as u16;
                    }
                });
            }
        }
        (width, height)
    }
}

impl<'a> Iterator for LayoutIter<'a> {
    type Item = PositionedGlyph<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        for c in &mut self.chars {
            if c.is_control() {
                if c == '\n' {
                    self.caret = rusttype::point(0.0, self.caret.y + self.advance_height);
                    self.last_glyph_id = None;
                }
                continue;
            }
            let base_glyph = self.font.glyph(c);
            if let Some(id) = self.last_glyph_id.take() {
                self.caret.x += self.font.pair_kerning(self.scale, id, base_glyph.id());
            }
            self.last_glyph_id = Some(base_glyph.id());
            let mut glyph = base_glyph.scaled(self.scale).positioned(self.caret);
            if let Some(bb) = glyph.pixel_bounding_box() {
                if bb.max.x as u32 > self.width {
                    self.caret = rusttype::point(0.0, self.caret.y + self.advance_height);
                    glyph = glyph.into_unpositioned().positioned(self.caret);
                    self.last_glyph_id = None;
                }
            }
            self.caret.x += glyph.unpositioned().h_metrics().advance_width;
            return Some(glyph);
        }
        None
    }
}

impl Index<TextId> for TextRenderer {
    type Output = Text;
    fn index(&self, id: TextId) -> &Text {
        self.text(id).expect("invalid text id")
    }
}

impl IndexMut<TextId> for TextRenderer {
    fn index_mut(&mut self, id: TextId) -> &mut Text {
        self.text_mut(id).expect("invalid text id")
    }
}

impl<'context> System<'context> for TextRenderer {
    type Dependencies = &'context Window;
    type Error = Error;

    fn create(window: &Window) -> Result<Self> {
        let mut font_bytes = Vec::with_capacity(1024 * 1024); // 1MB
        File::open(FONT_PATH)
            .and_then(|mut file| file.read_to_end(&mut font_bytes))
            .chain_err(|| ErrorKind(format!("Cannot read font at {}", FONT_PATH)))?;
        let bind_group_layout =
            window
                .device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Text bind group layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                multisampled: false,
                                view_dimension: wgpu::TextureViewDimension::D2,
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                });
        let pipeline_layout =
            window
                .device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Text pipeline layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });
        let shader_module = window
            .device()
            .create_shader_module(include_wgsl!("../../assets/shaders/text.wgsl"));
        let pipeline = window
            .device()
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Text pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader_module,
                    entry_point: "main_vs",
                    buffers: &[TextVertex::desc()],
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader_module,
                    entry_point: "main_fs",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: window.texture_format(),
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::all(),
                    })],
                }),
                multiview: None,
            });
        Ok(Self {
            font: Font::try_from_vec_and_index(font_bytes, 0)
                .ok_or_else(|| ErrorKind(format!("Failed to parse font at {:?}.", FONT_PATH)))?,
            slab: IdSlab::with_capacity(16),
            pixel_buffer: Vec::new(),
            bind_group_layout,
            pipeline,
        })
    }

    fn destroy(self, _window: &Window) -> Result<()> {
        if !self.slab.is_empty() {
            error!("Text leaked, {} instances.", self.slab.len());
        }
        Ok(())
    }

    fn debug_name() -> &'static str {
        "text_renderer"
    }
}

pub struct Text {
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    visible: bool,
}

impl Text {
    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }
}

struct LayoutIter<'a> {
    font: &'a Font<'static>,
    scale: Scale,
    width: u32,
    advance_height: f32,
    caret: FontPoint<f32>,
    last_glyph_id: Option<GlyphId>,
    chars: Recompositions<StrChars<'a>>,
}

impl<'a> LayoutIter<'a> {
    fn new(font: &'a Font<'static>, scale: Scale, width: u32, text: &'a str) -> Self {
        let v_metrics = font.v_metrics(scale);

        Self {
            font,
            scale,
            width,
            advance_height: v_metrics.ascent - v_metrics.descent + v_metrics.line_gap,
            caret: rusttype::point(0.0, v_metrics.ascent),
            last_glyph_id: None,
            chars: text.nfc(),
        }
    }
}

/// Hard-coded path to the TTF file to use for rendering debug text.
const FONT_PATH: &str = "assets/ttf/OpenSans-Regular.ttf";

/// Hard-coded font size.
const POINT_SIZE: f32 = 24.0;

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable, Default)]
struct TextVertex {
    a_pos: [f32; 2],
    a_uv: [f32; 2],
}

impl ShaderVertex for TextVertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        static ATTRIBUTES: OnceLock<Vec<wgpu::VertexAttribute>> = OnceLock::new();
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<TextVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &ATTRIBUTES.get_or_init(|| {
                vec![
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x2,
                        offset: offset_of!(TextVertex, a_pos) as u64,
                        shader_location: 0,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x2,
                        offset: offset_of!(TextVertex, a_uv) as u64,
                        shader_location: 1,
                    },
                ]
            }),
        }
    }
}

fn vertex(x: f32, y: f32, u: f32, v: f32) -> TextVertex {
    TextVertex {
        a_pos: [x, y],
        a_uv: [u, v],
    }
}
