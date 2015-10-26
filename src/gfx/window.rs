use error::Result;
use glium_sdl2::{SDL2Facade, DisplayBuild};
use glium::Frame;
use platform;
use sdl2;
use sdl2::Sdl;
use sdl2::video::GLProfile;


const WINDOW_TITLE: &'static str = "Rusty Doom v0.0.7 - Toggle mouse with backtick key (`))";
const OPENGL_DEPTH_SIZE: u8 = 24;


pub struct Window {
    facade: SDL2Facade,
    width: u32,
    height: u32,
}

impl Window {
    pub fn new(sdl: &Sdl, width: u32, height: u32) -> Result<Window> {
        let video = try!(sdl.video());
        let gl_attr = video.gl_attr();
        gl_attr.set_context_profile(GLProfile::Core);
        gl_attr.set_context_major_version(platform::GL_MAJOR_VERSION);
        gl_attr.set_context_minor_version(platform::GL_MINOR_VERSION);
        gl_attr.set_depth_size(OPENGL_DEPTH_SIZE);
        gl_attr.set_double_buffer(true);

        let facade = try!(video.window(WINDOW_TITLE, width as u32, height as u32)
            .position_centered()
            .opengl()
            .build_glium());

        sdl2::clear_error();
        Ok(Window {
           facade: facade,
           width: width,
           height: height,
        })
    }

    pub fn aspect_ratio(&self) -> f32 {
        self.width as f32 / self.height as f32
    }

    pub fn draw(&self) -> Frame {
        self.facade.draw()
    }

    pub fn facade(&self) -> &SDL2Facade {
        &self.facade
    }
}

