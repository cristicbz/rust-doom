use gl;
use gl::types::*;
use libc;
use math::{Vec2f, Vec2};
use std::mem;

pub struct Texture {
    target: GLenum,
    id: GLuint,
    width: uint,
    height: uint,
}

impl Texture {
    pub fn new(target: GLenum) -> Texture {
        let mut result = 0;
        check_gl_unsafe!(gl::GenTextures(1, &mut result));
        Texture { id: result, target: target, width: 0, height: 0}
    }

    pub fn set_filters_nearest(&mut self) -> &mut Texture {
        check_gl_unsafe!(gl::TexParameteri(self.target, gl::TEXTURE_MAG_FILTER,
                                    gl::NEAREST as GLint));
        check_gl_unsafe!(gl::TexParameteri(self.target, gl::TEXTURE_MIN_FILTER,
                                    gl::NEAREST as GLint));
        self
    }

    pub fn set_filters_linear(&mut self) -> &mut Texture {
        check_gl_unsafe!(gl::TexParameteri(self.target, gl::TEXTURE_MAG_FILTER,
                                    gl::LINEAR as GLint));
        check_gl_unsafe!(gl::TexParameteri(self.target, gl::TEXTURE_MIN_FILTER,
                                    gl::LINEAR as GLint));
        self
    }

    pub fn data_rgb_u8<T : Copy>(&mut self,
                       level: uint, width: uint, height: uint, data: &[T])
            -> &mut Texture {
        assert!(data.len() * mem::size_of::<T>() == (width * height * 3));
        check_gl_unsafe!(gl::TexImage2D(self.target,
                                        level as GLint,
                                        gl::RGB8 as GLint,
                                        width as GLsizei, height as GLsizei, 0,
                                        gl::RGB, gl::UNSIGNED_BYTE,
                                        data.as_ptr() as *const libc::c_void));
        self.width = width;
        self.height = height;
        self
    }

    pub fn data_red_u8<T : Copy>(&mut self,
                       level: uint, width: uint, height: uint, data: &[T])
            -> &mut Texture {
        assert!(data.len() * mem::size_of::<T>() == width * height);
        check_gl_unsafe!(gl::TexImage2D(self.target,
                                        level as GLint,
                                        gl::R8 as GLint,
                                        width as GLsizei, height as GLsizei, 0,
                                        gl::RED, gl::UNSIGNED_BYTE,
                                        data.as_ptr() as *const libc::c_void));
        self.width = width;
        self.height = height;
        self
    }

    pub fn data_rg_u8<T : Copy>(&mut self,
                       level: uint, width: uint, height: uint, data: &[T])
            -> &mut Texture {
        assert!(data.len() * mem::size_of::<T>() == (width * height * 2));
        check_gl_unsafe!(gl::TexImage2D(self.target,
                                        level as GLint,
                                        gl::RG8 as GLint,
                                        width as GLsizei, height as GLsizei, 0,
                                        gl::RG, gl::UNSIGNED_BYTE,
                                        data.as_ptr() as *const libc::c_void));
        self.width = width;
        self.height = height;
        self
    }

    pub fn bind(&self, unit: GLenum) {
        check_gl_unsafe!(gl::ActiveTexture(unit));
        check_gl_unsafe!(gl::BindTexture(self.target, self.id));
    }

    pub fn unbind(&self, unit: GLenum) {
        check_gl_unsafe!(gl::ActiveTexture(unit));
        check_gl_unsafe!(gl::BindTexture(self.target, 0));
    }

    pub fn width(&self) -> uint { self.width }
    pub fn height(&self) -> uint { self.height }
    pub fn size_as_vec(&self) -> Vec2f { Vec2::new(self.width as f32,
                                                   self.height as f32) }
}
impl Drop for Texture {
    fn drop(&mut self) {
        check_gl_unsafe!(gl::DeleteTextures(1, &self.id));
    }
}
