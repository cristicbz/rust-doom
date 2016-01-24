use glium::{DrawParameters, Program};
use glium::Frame;
use glium::index::{NoIndices, PrimitiveType};
use glium::{Blend, Surface, VertexBuffer};
use glium::texture::{ClientFormat, RawImage2d, Texture2d};
use math::Vec2f;
use sdl2::pixels::Color;
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;
use sdl2::render::BlendMode;
use sdl2::surface::Surface as SdlSurface;
use sdl2_ttf as ttf;
use sdl2_ttf::Error as TtfError;
use sdl2_ttf::Font;
use sdl2_ttf::Sdl2TtfContext;
use slab::Slab;
use std::borrow::Cow;
use std::cmp;
use std::error::Error as StdError;
use std::fmt::{Display, Formatter};
use std::fmt::Result as FmtResult;
use std::ops::{Index, IndexMut};
use Window;

/// A handle to a piece of text created with a `TextRenderer`.
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct TextId(usize);

/// Handles rendering of debug text to OpenGL
pub struct TextRenderer {
    font: Font,
    slab: Slab<Text, usize>,
    program: Program,
    draw_params: DrawParameters<'static>,
}

impl TextRenderer {
    pub fn new(win: &Window) -> Result<TextRenderer, Error> {
        CONTEXT.as_ref().unwrap();
        Ok(TextRenderer {
            font: Font::from_file(FONT_PATH.as_ref(), POINT_SIZE).unwrap(),
            slab: Slab::new(MAX_TEXT),
            program: Program::from_source(win.facade(), VERTEX_SRC, FRAGMENT_SRC, None).unwrap(),
            draw_params: DrawParameters {
                blend: Blend::alpha_blending(),
                ..DrawParameters::default()
            },
        })
    }

    pub fn insert(&mut self, win: &Window, text: &str, pos: Vec2f, padding: u32) -> TextId {
        let surface = self.text_to_surface(text, padding).unwrap();
        let texture = surface.with_lock(|pixels| {
            Texture2d::new(win.facade(),
                           RawImage2d {
                               data: Cow::Borrowed(pixels),
                               width: surface.width(),
                               height: surface.height(),
                               format: ClientFormat::U8U8U8U8,
                           })
                .unwrap()
        });
        let (w, h) = (surface.width() as f32 / win.width() as f32 * 2.0,
                      surface.height() as f32 / win.height() as f32 * 2.0);
        let (x, y) = (pos[0] * 2.0 - 1.0, 1.0 - pos[1] * 2.0 - h);
        let text = Text {
            buffer: VertexBuffer::immutable(win.facade(),
                                            &[vertex(x, y, 0.0, 1.0),
                                              vertex(x, y + h, 0.0, 0.0),
                                              vertex(x + w, y, 1.0, 1.0),
                                              vertex(x + w, y + h, 1.0, 0.0)])
                        .unwrap(),
            texture: texture,
            visible: true,
        };
        TextId(self.slab.insert(text).ok().expect("too many text objects."))
    }

    pub fn text(&self, id: TextId) -> Option<&Text> {
        self.slab.get(id.0)
    }

    pub fn text_mut(&mut self, id: TextId) -> Option<&mut Text> {
        self.slab.get_mut(id.0)
    }

    pub fn render(&self, frame: &mut Frame) -> Result<(), Error> {
        for text in &self.slab {
            if !text.visible {
                continue;
            }
            let uniforms = uniform! {
                u_tex: &text.texture,
            };
            frame.draw(&text.buffer,
                       NoIndices(PrimitiveType::TriangleStrip),
                       &self.program,
                       &uniforms,
                       &self.draw_params)
                 .unwrap();
        }
        Ok(())
    }

    fn text_to_surface(&self, text: &str, padding: u32) -> Result<SdlSurface<'static>, Error> {
        let wrap_length = text.lines()
                              .filter_map(|line| self.font.size(line).ok())
                              .map(|size| size.0)
                              .fold(0, cmp::max) + 10;
        let mut text = self.font
                           .render(text,
                                   ttf::blended_wrapped(Color::RGBA(255, 255, 255, 255),
                                                        wrap_length))
                           .unwrap();
        let mut surface = SdlSurface::new(text.width() + padding * 2,
                                          text.height() + padding * 2,
                                          PixelFormatEnum::ARGB8888)
                              .unwrap();
        surface.set_blend_mode(BlendMode::None).unwrap();
        surface.fill_rect(None, Color::RGBA(0, 0, 0, 128)).unwrap();
        surface.set_blend_mode(BlendMode::Blend).unwrap();
        text.set_blend_mode(BlendMode::Blend).unwrap();
        text.blit(None,
                  &mut surface,
                  Some(Rect::new_unwrap(padding as i32,
                                        padding as i32,
                                        text.width(),
                                        text.height())))
            .unwrap();
        Ok(surface)
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

#[derive(Debug)]
pub enum Error {}
impl StdError for Error {
    fn description(&self) -> &str {
        "FIXME: Text error."
    }

    fn cause(&self) -> Option<&StdError> {
        None
    }
}

impl Display for Error {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        write!(fmt, "{}", self.description())
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

// Use a lazy static to initialise the ttf context only once.
lazy_static! {
    static ref CONTEXT: Result<Sdl2TtfContext, TtfError> = {
        info!("Initialising SDL2_ttf: {}", ttf::get_linked_version());
        ttf::init()
    };
}

/// Hard-coded path to the TTF file to use for rendering debug text.
const FONT_PATH: &'static str = "ttf/OpenSans-Regular.ttf";

/// Hard-coded font size.
const POINT_SIZE: i32 = 18;

/// Hard-coded maximum number of `Text objects`.
const MAX_TEXT: usize = 32;

const VERTEX_SRC: &'static str = r#"
    #version 140
    in vec2 a_pos;
    in vec2 a_uv;
    out vec2 v_uv;
    void main() {
        v_uv = a_uv;
        gl_Position = vec4(a_pos, 0.0, 1.0);
    }
"#;

const FRAGMENT_SRC: &'static str = r#"
    #version 140
    uniform sampler2D u_tex;
    in vec2 v_uv;
    out vec4 color;
    void main() { color = texture(u_tex, v_uv); }
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
