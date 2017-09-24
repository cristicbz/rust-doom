
use super::errors::{Result, ResultExt, ErrorKind, Error};
use super::platform;
use super::system::System;
use glium::{Frame, Surface};
use glium_sdl2::{DisplayBuild, SDL2Facade};
use sdl2;
use sdl2::Sdl;
use sdl2::video::GLProfile;

const OPENGL_DEPTH_SIZE: u8 = 24;

pub struct WindowConfig {
    pub width: u32,
    pub height: u32,
    pub title: String,
}

pub struct Window {
    sdl: Sdl,
    facade: SDL2Facade,
    width: u32,
    height: u32,
}

impl Window {
    pub fn sdl(&self) -> &Sdl {
        &self.sdl
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn aspect_ratio(&self) -> f32 {
        self.width as f32 / self.height as f32
    }

    pub fn draw(&self) -> Frame {
        let mut frame = self.facade.draw();
        frame.clear_all_srgb((0.06, 0.07, 0.09, 0.0), 1.0, 0);
        frame
    }

    pub fn facade(&self) -> &SDL2Facade {
        &self.facade
    }
}

impl<'context> System<'context> for Window {
    type Dependencies = &'context WindowConfig;
    type Error = Error;

    fn create(config: &'context WindowConfig) -> Result<Self> {
        let sdl = sdl2::init().map_err(ErrorKind::Sdl)?;
        let video = sdl.video().map_err(ErrorKind::Sdl)?;
        let gl_attr = video.gl_attr();
        gl_attr.set_context_profile(GLProfile::Core);
        gl_attr.set_context_major_version(platform::GL_MAJOR_VERSION);
        gl_attr.set_context_minor_version(platform::GL_MINOR_VERSION);
        gl_attr.set_depth_size(OPENGL_DEPTH_SIZE);
        gl_attr.set_double_buffer(true);

        let facade = video
            .window(&config.title, config.width, config.height)
            .position_centered()
            .opengl()
            .resizable()
            .build_glium()
            .chain_err(|| ErrorKind::CreateWindow(config.width, config.height))?;

        sdl2::clear_error();
        Ok(Window {
            sdl: sdl,
            facade: facade,
            width: config.width,
            height: config.height,
        })
    }

    fn debug_name() -> &'static str {
        "window"
    }
}
