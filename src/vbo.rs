use check_gl;
use gl;
use gl::types::{GLuint, GLenum, GLsizeiptr};
use libc;
use std::cell::Cell;
use std::mem;

pub struct VertexBuffer {
    id: GLuint,
    target: GLenum,
    length: uint,
    bound: Cell<bool>,
}

impl VertexBuffer {
    pub fn new(target: GLenum) -> VertexBuffer {
        let mut id : GLuint = 0;
        check_gl_unsafe!(gl::GenBuffers(1, &mut id));
        assert!(id != 0 && target != 0);
        VertexBuffer {
            id: id,
            target: target,
            length: 0,
            bound: Cell::new(false)
        }
    }

    pub fn new_with_data<T: Copy>(target: GLenum, usage: GLenum, data: &[T])
            -> VertexBuffer {
        let mut buf = VertexBuffer::new(target);
        buf.bind_mut().buffer_data(usage, data).unbind();
        buf
    }

    pub fn bind_mut(&mut self) -> &mut VertexBuffer {
        check_gl!(gl::BindBuffer(self.target, self.id));
        self.set_bound(true);
        self
    }

    pub fn bind(&self) -> &VertexBuffer {
        check_gl!(gl::BindBuffer(self.target, self.id));
        self.set_bound(true);
        self
    }

    pub fn unbind(&self) -> &VertexBuffer {
        self.assert_bound(true);
        check_gl!(gl::BindBuffer(self.target, 0));
        self.set_bound(false);
        self
    }

    pub fn len(&self) -> uint { self.length }

    pub fn buffer_data<T : Copy>(&mut self, usage: GLenum, data: &[T])
            -> &mut VertexBuffer {
        self.assert_bound(true);
        self.length = data.len();
        check_gl_unsafe!(gl::BufferData(
                self.target, (data.len() * mem::size_of::<T>()) as GLsizeiptr,
                data.as_ptr() as *const libc::c_void, usage));
        self
    }

    fn set_bound(&self, is_bound: bool) {
        self.bound.set(is_bound);
    }

    fn assert_bound(&self, is_bound: bool) {
        assert!(self.bound.get() == is_bound);
    }
}

impl Drop for VertexBuffer {
    fn drop(&mut self) {
        if self.id != 0 {
            check_gl_unsafe!(gl::DeleteBuffers(1, &self.id));
        }
    }
}
