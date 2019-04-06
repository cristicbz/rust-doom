#![cfg_attr(feature = "cargo-clippy", allow(clippy::forget_copy))]

use super::system::System;
use super::window::Window;
use failchain::{ChainErrorKind, ResultExt, UnboxedError};
use failure::Fail;
use glium::index::{NoIndices, PrimitiveType};
use glium::texture::{ClientFormat, RawImage2d, Texture2d};
use glium::{
    implement_vertex, uniform, Blend, DrawParameters, Frame, Program, Surface, VertexBuffer,
};
use idcontain::{Id, IdSlab};
use log::{debug, error};
use math::Pnt2f;
use rusttype::{self, Font, FontCollection, GlyphId, Point as FontPoint, PositionedGlyph, Scale};
use std::borrow::Cow;
use std::fs::File;
use std::io::Read;
use std::ops::{Index, IndexMut};
use std::result::Result as StdResult;
use std::str::Chars as StrChars;
use unicode_normalization::{Recompositions, UnicodeNormalization};

/// A handle to a piece of text created with a `TextRenderer`.
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct TextId(Id<Text>);

/// Handles rendering of debug text to `OpenGL`.
pub struct TextRenderer {
    font: Font<'static>,
    slab: IdSlab<Text>,
    program: Program,
    draw_params: DrawParameters<'static>,
    pixel_buffer: Vec<u16>,
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
        let (width, height) = self.rasterise(text, padding).unwrap();
        let texture = Texture2d::new(
            win.facade(),
            RawImage2d {
                data: Cow::Borrowed(&self.pixel_buffer),
                width,
                height,
                format: ClientFormat::U8U8,
            },
        )
        .unwrap();
        let (w, h) = (
            width as f32 / win.width() as f32 * 2.0,
            height as f32 / win.height() as f32 * 2.0,
        );
        let (x, y) = (pos.x * 2.0 - 1.0, 1.0 - pos.y * 2.0 - h);
        let text = Text {
            buffer: VertexBuffer::immutable(
                win.facade(),
                &[
                    vertex(x, y, 0.0, 1.0),
                    vertex(x, y + h, 0.0, 0.0),
                    vertex(x + w, y, 1.0, 1.0),
                    vertex(x + w, y + h, 1.0, 0.0),
                ],
            )
            .unwrap(),
            texture,
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

    pub fn render(&self, frame: &mut Frame) -> Result<()> {
        for text in &self.slab {
            if !text.visible {
                continue;
            }
            let uniforms = uniform! {
                u_tex: &text.texture,
            };
            frame
                .draw(
                    &text.buffer,
                    NoIndices(PrimitiveType::TriangleStrip),
                    &self.program,
                    &uniforms,
                    &self.draw_params,
                )
                .unwrap();
        }
        Ok(())
    }

    fn rasterise(&mut self, text: &str, padding: u32) -> Result<(u32, u32)> {
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
        Ok((width, height))
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
        Ok(Self {
            font: FontCollection::from_bytes(font_bytes)
                .chain_err(|| ErrorKind(format!("Failed to parse font at {:?}.", FONT_PATH)))?
                .font_at(0)
                .chain_err(|| ErrorKind(format!("No fonts in {:?}.", FONT_PATH)))?,
            slab: IdSlab::with_capacity(16),
            program: Program::from_source(window.facade(), VERTEX_SRC, FRAGMENT_SRC, None).unwrap(),
            draw_params: DrawParameters {
                blend: Blend::alpha_blending(),
                ..DrawParameters::default()
            },
            pixel_buffer: Vec::new(),
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
    texture: Texture2d,
    buffer: VertexBuffer<TextVertex>,
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

const VERTEX_SRC: &str = r#"
    #version 140
    in vec2 a_pos;
    in vec2 a_uv;
    out vec2 v_uv;
    void main() {
        v_uv = a_uv;
        gl_Position = vec4(a_pos, 0.0, 1.0);
    }
"#;

const FRAGMENT_SRC: &str = r#"
    #version 140
    uniform sampler2D u_tex;
    in vec2 v_uv;
    out vec4 color;
    void main() {
        vec4 tex_color = texture(u_tex, v_uv);
        color = vec4(tex_color.g, tex_color.g, tex_color.g, tex_color.r);
    }
"#;

#[repr(C)]
#[derive(Copy, Clone)]
struct TextVertex {
    a_pos: [f32; 2],
    a_uv: [f32; 2],
}

implement_vertex!(TextVertex, a_pos, a_uv);

fn vertex(x: f32, y: f32, u: f32, v: f32) -> TextVertex {
    TextVertex {
        a_pos: [x, y],
        a_uv: [u, v],
    }
}
