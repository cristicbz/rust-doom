use error::Result;
use error::Error::Sdl as SdlError;
use sdl2::Sdl;
use sdl2::video::Window as SdlWindow;
use sdl2::video::{gl_attr, GLProfile, GLContext};
use libc::c_void;
use gl;
use sdl2;


const WINDOW_TITLE: &'static str = "Rusty Doom v0.0.7 - Toggle mouse with backtick key (`))";
const OPENGL_DEPTH_SIZE: u8 = 24;


pub struct Window {
    window: SdlWindow,
    width: u32,
    height: u32,
    _context: GLContext,
}

impl Window {
    pub fn new(sdl: &Sdl, width: u32, height: u32) -> Result<Window> {
        gl_attr::set_context_profile(GLProfile::Core);
        gl_attr::set_context_major_version(gl::platform::GL_MAJOR_VERSION);
        gl_attr::set_context_minor_version(gl::platform::GL_MINOR_VERSION);
        gl_attr::set_depth_size(OPENGL_DEPTH_SIZE);
        gl_attr::set_double_buffer(true);

        let window = try!(sdl.window(WINDOW_TITLE, width as u32, height as u32)
            .position_centered()
            .opengl()
            .build()
            .map_err(SdlError));

        let context = try!(window.gl_create_context().map_err(SdlError));
        sdl2::clear_error();
        gl::load_with(|name| {
            sdl2::video::gl_get_proc_address(name) as *const c_void
        });
        let mut vao_id = 0;
        check_gl_unsafe!(gl::GenVertexArrays(1, &mut vao_id));
        check_gl_unsafe!(gl::BindVertexArray(vao_id));
        check_gl_unsafe!(gl::ClearColor(0.06, 0.07, 0.09, 0.0));
        check_gl_unsafe!(gl::Enable(gl::DEPTH_TEST));
        check_gl_unsafe!(gl::DepthFunc(gl::LESS));
        Ok(Window {
           window: window,
           width: width,
           height: height,
           _context: context,
        })
    }

    pub fn aspect_ratio(&self) -> f32 {
        self.width as f32 / self.height as f32
    }

    pub fn swap_buffers(&self) {
        self.window.gl_swap_window();
    }

    pub fn clear(&self) {
        check_gl_unsafe!(gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT));
    }
}

