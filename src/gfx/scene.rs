use error::{NeededBy, Result};
use glium::{BackfaceCullingMode, Depth, DepthTest, DrawParameters, Frame, Program, Surface};
use glium::index::{NoIndices, PrimitiveType};
use glium::texture::{ClientFormat, RawImage2d, Texture2d};
use glium::texture::buffer_texture::{BufferTexture, BufferTextureType};
use glium::uniforms::{MagnifySamplerFilter, MinifySamplerFilter, SamplerBehavior};
use glium::uniforms::SamplerWrapFunction;
use glium::uniforms::{AsUniformValue, UniformValue, Uniforms};
use glium::program::ProgramCreationInput;
use math::{Mat4, Vec2};
use platform;
use std::borrow::Cow;
use std::fs::File;
use std::io::Read;
use std::io::Result as IoResult;
use std::path::{Path, PathBuf};
use vertex::{DecorBufferBuilder, FlatBufferBuilder, SkyBufferBuilder, WallBufferBuilder};
use vertex::{SkyBuffer, SpriteBuffer, StaticBuffer};
use window::Window;

pub struct SceneBuilder<'window> {
    window: &'window Window,
    root_shader_path: PathBuf,

    palette: Option<Texture2d>,

    sky_program: Option<Program>,
    sky_texture: Option<Texture2d>,
    sky_buffer: SkyBufferBuilder,
    tiled_band_size: Option<f32>,

    static_program: Option<Program>,
    flats_texture: Option<Texture2d>,
    walls_texture: Option<Texture2d>,
    flats_buffer: FlatBufferBuilder,
    walls_buffer: WallBufferBuilder,

    sprite_program: Option<Program>,
    decors_texture: Option<Texture2d>,
    decors_buffer: DecorBufferBuilder,
}

impl<'window> SceneBuilder<'window> {
    pub fn new(window: &'window Window, root_shader_path: PathBuf) -> SceneBuilder {
        SceneBuilder {
            window: window,
            root_shader_path: root_shader_path,
            palette: None,
            sky_program: None,
            sky_texture: None,
            sky_buffer: SkyBufferBuilder::new(),
            tiled_band_size: None,
            static_program: None,
            flats_texture: None,
            walls_texture: None,
            flats_buffer: FlatBufferBuilder::new(),
            walls_buffer: WallBufferBuilder::new(),
            sprite_program: None,
            decors_texture: None,
            decors_buffer: DecorBufferBuilder::new(),
        }
    }

    pub fn palette(&mut self, pixels: &[u8]) -> Result<&mut Self> {
        assert_eq!(pixels.len() % (256 * 3), 0);
        self.palette = Some(try!(Texture2d::new(self.window.facade(),
                                                RawImage2d {
                                                    data: Cow::Borrowed(pixels),
                                                    width: 256,
                                                    height: (pixels.len() / (256 * 3)) as u32,
                                                    format: ClientFormat::U8U8U8,
                                                })
                                     .needed_by("palette texture")));
        Ok(self)
    }

    pub fn sky_program(&mut self, name: &str) -> Result<&mut Self> {
        self.sky_program = Some(try!(self.load_program(name)));
        Ok(self)
    }

    pub fn static_program(&mut self, name: &str) -> Result<&mut Self> {
        self.static_program = Some(try!(self.load_program(name)));
        Ok(self)
    }

    pub fn sprite_program(&mut self, name: &str) -> Result<&mut Self> {
        self.sprite_program = Some(try!(self.load_program(name)));
        Ok(self)
    }

    pub fn no_sky_texture(&mut self) -> Result<&mut Self> {
        debug!("Setting no sky texture.");
        self.sky_texture = Some(try!(Texture2d::empty(self.window.facade(), 1, 1)
                                         .needed_by("empty sky texture")));
        Ok(self)
    }

    pub fn sky_texture(&mut self, pixels: &[u16], size: Vec2<usize>) -> Result<&mut Self> {
        debug!("Setting sky texture: pixels={}, size={:?}",
               pixels.len(),
               size);
        self.sky_texture = Some(try!(Texture2d::new(self.window.facade(),
                                                    RawImage2d {
                                                        data: Cow::Borrowed(pixels),
                                                        width: size[0] as u32,
                                                        height: size[1] as u32,
                                                        format: ClientFormat::U8U8,
                                                    })
                                         .needed_by("sky texture")));
        Ok(self)
    }

    pub fn flats_texture(&mut self, pixels: &[u8], size: Vec2<usize>) -> Result<&mut Self> {
        debug!("Setting flats texture: pixels={}, size={:?}",
               pixels.len(),
               size);
        self.flats_texture = Some(try!(Texture2d::new(self.window.facade(),
                                                      RawImage2d {
                                                          data: Cow::Borrowed(pixels),
                                                          width: size[0] as u32,
                                                          height: size[1] as u32,
                                                          format: ClientFormat::U8,
                                                      })
                                           .needed_by("flats atlas texture")));
        Ok(self)
    }

    pub fn walls_texture(&mut self, pixels: &[u16], size: Vec2<usize>) -> Result<&mut Self> {
        debug!("Setting walls texture: pixels={}, size={:?}",
               pixels.len(),
               size);
        self.walls_texture = Some(try!(Texture2d::new(self.window.facade(),
                                                      RawImage2d {
                                                          data: Cow::Borrowed(pixels),
                                                          width: size[0] as u32,
                                                          height: size[1] as u32,
                                                          format: ClientFormat::U8U8,
                                                      })
                                           .needed_by("walls atlas texture")));
        Ok(self)
    }

    pub fn decors_texture(&mut self, pixels: &[u16], size: Vec2<usize>) -> Result<&mut Self> {
        debug!("Setting decors texture: pixels={}, size={:?}",
               pixels.len(),
               size);
        self.decors_texture = Some(try!(Texture2d::new(self.window.facade(),
                                                       RawImage2d {
                                                           data: Cow::Borrowed(pixels),
                                                           width: size[0] as u32,
                                                           height: size[1] as u32,
                                                           format: ClientFormat::U8U8,
                                                       })
                                            .needed_by("decors texture")));
        Ok(self)
    }

    pub fn sky_buffer(&mut self) -> &mut SkyBufferBuilder {
        &mut self.sky_buffer
    }

    pub fn flats_buffer(&mut self) -> &mut FlatBufferBuilder {
        &mut self.flats_buffer
    }

    pub fn walls_buffer(&mut self) -> &mut WallBufferBuilder {
        &mut self.walls_buffer
    }

    pub fn decors_buffer(&mut self) -> &mut DecorBufferBuilder {
        &mut self.decors_buffer
    }

    pub fn tiled_band_size(&mut self, size: f32) -> &mut Self {
        self.tiled_band_size = Some(size);
        self
    }

    pub fn build(self) -> Result<Scene> {
        Ok(Scene {
            draw_params: DrawParameters {
                depth: Depth {
                    test: DepthTest::IfLess,
                    write: true,
                    ..Depth::default()
                },
                backface_culling: BackfaceCullingMode::CullClockwise,
                ..DrawParameters::default()
            },
            projection: mat4_to_uniform(&Mat4::new_identity()),
            modelview: mat4_to_uniform(&Mat4::new_identity()),
            time: 0.0f32,
            lights: try!(BufferTexture::empty_persistent(self.window.facade(),
                                                         256,
                                                         BufferTextureType::Float)
                             .needed_by("lights buffer")),
            palette: self.palette.expect("missing palette from SceneBuilder"),
            sky_program: self.sky_program.expect("missing sky program from SceneBuilder"),
            sky_texture: self.sky_texture.expect("missing sky texture from SceneBuilder"),
            sky_buffer: try!(self.sky_buffer.build(self.window)),
            tiled_band_size: self.tiled_band_size.expect("missing tiled band from SceneBuilder"),

            static_program: self.static_program.expect("missing static program from SceneBuilder"),
            flats_texture: self.flats_texture.expect("missing flats texture from SceneBuilder"),
            walls_texture: self.walls_texture.expect("missing walls texture from SceneBuilder"),
            flats_buffer: try!(self.flats_buffer.build(&self.window)),
            walls_buffer: try!(self.walls_buffer.build(&self.window)),

            sprite_program: self.sprite_program.expect("missing sprite program from SceneBuilder"),
            decors_texture: self.decors_texture.expect("missing decors texture from SceneBuilder"),
            decors_buffer: try!(self.decors_buffer.build(&self.window)),
        })
    }

    fn load_program(&self, name: &str) -> Result<Program> {
        let mut frag_path = self.root_shader_path.clone();
        frag_path.push(name);

        let mut vert_path = frag_path.clone();
        frag_path.set_extension("frag");
        vert_path.set_extension("vert");

        let mut frag_src = format!("#version {}\n", platform::GLSL_VERSION_STRING);
        let mut vert_src = frag_src.clone();

        debug!("Loading shader: {} (from {:?} and {:?})",
               name,
               frag_path,
               vert_path);
        try!(read_utf8_file(&frag_path, &mut frag_src));
        try!(read_utf8_file(&vert_path, &mut vert_src));
        let program = try!(Program::new(self.window.facade(),
                                        ProgramCreationInput::SourceCode {
                                            vertex_shader: &vert_src,
                                            tessellation_control_shader: None,
                                            tessellation_evaluation_shader: None,
                                            geometry_shader: None,
                                            fragment_shader: &frag_src,
                                            transform_feedback_varyings: None,
                                            outputs_srgb: true,
                                            uses_point_size: false,
                                        })
                               .needed_by(name));
        debug!("Shader '{}' loaded successfully", name);
        Ok(program)
    }
}

pub struct Scene {
    draw_params: DrawParameters<'static>,
    projection: UniformValue<'static>,
    modelview: UniformValue<'static>,
    time: f32,
    lights: BufferTexture<u8>,

    palette: Texture2d,

    sky_program: Program,
    sky_texture: Texture2d,
    sky_buffer: SkyBuffer,
    tiled_band_size: f32,

    static_program: Program,
    flats_texture: Texture2d,
    walls_texture: Texture2d,
    flats_buffer: StaticBuffer,
    walls_buffer: StaticBuffer,

    sprite_program: Program,
    decors_texture: Texture2d,
    decors_buffer: SpriteBuffer,
}

impl Scene {
    pub fn set_projection(&mut self, value: &Mat4) {
        self.projection = mat4_to_uniform(value);
    }

    pub fn set_modelview(&mut self, value: &Mat4) {
        self.modelview = mat4_to_uniform(value);
    }

    pub fn set_lights<F>(&mut self, writer: F)
        where F: FnOnce(&mut [u8])
    {
        writer(&mut *self.lights.map())
    }

    pub fn render(&mut self, frame: &mut Frame, delta_time: f32) -> Result<()> {
        self.time += delta_time;

        try!(StaticStep::flats(self).render(frame));
        try!(StaticStep::walls(self).render(frame));
        try!(SpriteStep::decors(self).render(frame));
        try!(SkyStep::new(self).render(frame));

        Ok(())
    }
}

pub struct StaticStep<'a> {
    name: &'static str,
    scene: &'a Scene,
    texture: &'a Texture2d,
    program: &'a Program,
    buffer: &'a StaticBuffer,
}

impl<'scene> StaticStep<'scene> {
    pub fn flats(scene: &'scene Scene) -> StaticStep<'scene> {
        StaticStep {
            scene: scene,
            texture: &scene.flats_texture,
            buffer: &scene.flats_buffer,
            program: &scene.static_program,
            name: "flats render step",
        }
    }

    pub fn walls(scene: &'scene Scene) -> StaticStep<'scene> {
        StaticStep {
            scene: scene,
            texture: &scene.walls_texture,
            buffer: &scene.walls_buffer,
            program: &scene.static_program,
            name: "walls render step",
        }
    }

    pub fn render(self, frame: &mut Frame) -> Result<()> {
        try!(frame.draw(self.buffer,
                        NoIndices(PrimitiveType::TrianglesList),
                        &self.program,
                        &self,
                        &self.scene.draw_params)
                  .needed_by(self.name));
        Ok(())
    }
}

impl<'scene> Uniforms for StaticStep<'scene> {
    fn visit_values<'a, F>(&'a self, mut set_uniform: F)
        where F: FnMut(&str, UniformValue<'a>)
    {
        set_uniform("u_modelview", self.scene.modelview);
        set_uniform("u_projection", self.scene.projection);
        set_uniform("u_time", UniformValue::Float(self.scene.time));
        set_uniform("u_lights", self.scene.lights.as_uniform_value());
        set_uniform("u_palette",
                    UniformValue::Texture2d(&self.scene.palette, PALETTE_SAMPLER));
        set_uniform("u_atlas", UniformValue::Texture2d(self.texture, SAMPLER));
        set_uniform("u_atlas_size",
                    UniformValue::Vec2([self.texture.get_width() as f32,
                                        self.texture.get_height()
                                                    .expect("1d static atlas") as f32]));
    }
}


pub struct SkyStep<'a>(&'a Scene);

impl<'scene> SkyStep<'scene> {
    pub fn new(scene: &'scene Scene) -> SkyStep<'scene> {
        SkyStep(scene)
    }

    pub fn render(self, frame: &mut Frame) -> Result<()> {
        try!(frame.draw(&self.0.sky_buffer,
                        NoIndices(PrimitiveType::TrianglesList),
                        &self.0.sky_program,
                        &self,
                        &self.0.draw_params)
                  .needed_by("sky render step"));
        Ok(())
    }
}

impl<'scene> Uniforms for SkyStep<'scene> {
    fn visit_values<'a, F>(&'a self, mut set_uniform: F)
        where F: FnMut(&str, UniformValue<'a>)
    {
        set_uniform("u_modelview", self.0.modelview);
        set_uniform("u_projection", self.0.projection);
        set_uniform("u_time", UniformValue::Float(self.0.time));
        set_uniform("u_palette",
                    UniformValue::Texture2d(&self.0.palette, PALETTE_SAMPLER));
        set_uniform("u_texture",
                    UniformValue::Texture2d(&self.0.sky_texture, SAMPLER));
        set_uniform("u_tiled_band_size",
                    UniformValue::Float(self.0.tiled_band_size));
    }
}


pub struct SpriteStep<'scene> {
    scene: &'scene Scene,
    texture: &'scene Texture2d,
    program: &'scene Program,
    buffer: &'scene SpriteBuffer,
}

impl<'scene> SpriteStep<'scene> {
    pub fn decors(scene: &'scene Scene) -> SpriteStep<'scene> {
        SpriteStep {
            scene: scene,
            texture: &scene.decors_texture,
            program: &scene.sprite_program,
            buffer: &scene.decors_buffer,
        }
    }
}

impl<'scene> Uniforms for SpriteStep<'scene> {
    fn visit_values<'a, F>(&'a self, mut set_uniform: F)
        where F: FnMut(&str, UniformValue<'a>)
    {
        set_uniform("u_modelview", self.scene.modelview);
        set_uniform("u_projection", self.scene.projection);
        set_uniform("u_lights", self.scene.lights.as_uniform_value());
        set_uniform("u_time", UniformValue::Float(self.scene.time));
        set_uniform("u_palette",
                    UniformValue::Texture2d(&self.scene.palette, PALETTE_SAMPLER));
        set_uniform("u_atlas", UniformValue::Texture2d(self.texture, SAMPLER));
        set_uniform("u_atlas_size",
                    UniformValue::Vec2([self.texture.get_width() as f32,
                                        self.texture.get_height()
                                                    .expect("1d sprite atlas") as f32]));
    }
}

impl<'scene> SpriteStep<'scene> {
    fn render(self, frame: &mut Frame) -> Result<()> {
        try!(frame.draw(self.buffer,
                        NoIndices(PrimitiveType::TrianglesList),
                        self.program,
                        &self,
                        &self.scene.draw_params)
                  .needed_by("sprite render step"));
        Ok(())
    }
}

fn mat4_to_uniform(m: &Mat4) -> UniformValue<'static> {
    UniformValue::Mat4([[m[0][0], m[0][1], m[0][2], m[0][3]],
                        [m[1][0], m[1][1], m[1][2], m[1][3]],
                        [m[2][0], m[2][1], m[2][2], m[2][3]],
                        [m[3][0], m[3][1], m[3][2], m[3][3]]])
}

fn read_utf8_file(path: &Path, into: &mut String) -> IoResult<()> {
    try!(File::open(path)).read_to_string(into).map(|_| ())
}


const SAMPLER: Option<SamplerBehavior> = Some(SamplerBehavior {
    wrap_function: (SamplerWrapFunction::Repeat,
                    SamplerWrapFunction::Repeat,
                    SamplerWrapFunction::Repeat),
    minify_filter: MinifySamplerFilter::Nearest,
    magnify_filter: MagnifySamplerFilter::Nearest,
    max_anisotropy: 1,
});

const PALETTE_SAMPLER: Option<SamplerBehavior> = Some(SamplerBehavior {
    wrap_function: (SamplerWrapFunction::Clamp,
                    SamplerWrapFunction::Clamp,
                    SamplerWrapFunction::Clamp),
    minify_filter: MinifySamplerFilter::Nearest,
    magnify_filter: MagnifySamplerFilter::Nearest,
    max_anisotropy: 1,
});
