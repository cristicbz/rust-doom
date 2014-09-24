use check_gl;
use gl;
use gl::types::{GLint, GLuint, GLenum, GLsizeiptr};
use libc;
use libc::c_void;
use std::mem;
use std::vec;

type IndexType = u16;

#[deriving(Clone)]
struct VertexAttribute {
    layout: GLuint,
    gl_type: GLenum,
    size: GLint,
    normalized: u8,
    offset: *const c_void
}

pub struct BufferBuilder<VertexType: Copy> {
    attributes: Vec<VertexAttribute>,
    used_layouts: Vec<bool>,
    vertex_size: uint
}
impl<VertexType: Copy>  BufferBuilder<VertexType> {
    pub fn new(capacity: uint) -> BufferBuilder<VertexType> {
        BufferBuilder {
            attributes: Vec::with_capacity(capacity),
            used_layouts: Vec::with_capacity(capacity),
            vertex_size: 0,
        }
    }

    pub fn max_attribute_size_left(&self) -> uint {
        let final_size = mem::size_of::<VertexType>();
        assert!(final_size >= self.vertex_size);
        final_size - self.vertex_size
    }

    pub fn attribute_f32(&mut self, layout: uint, offset: *const c_void)
            -> &mut BufferBuilder<VertexType> {
        self.new_attribute(layout, gl::FLOAT, 1, false, offset)
    }

    pub fn attribute_vec2f(&mut self, layout: uint, offset: *const c_void)
            -> &mut BufferBuilder<VertexType> {
        self.new_attribute(layout, gl::FLOAT, 2, false, offset)
    }

    pub fn attribute_vec3f(&mut self, layout: uint, offset: *const c_void)
            -> &mut BufferBuilder<VertexType> {
        self.new_attribute(layout, gl::FLOAT, 3, false, offset)
    }

    pub fn new_attribute(&mut self, layout: uint, gl_type: GLenum,
                         size: uint, normalized: bool, offset: *const c_void)
            -> &mut BufferBuilder<VertexType> {
        assert!(self.max_attribute_size_left() >= size * match gl_type {
            gl::FLOAT => mem::size_of::<f32>(),
            _ => fail!("Unsupported attribute type.")
        });
        assert!(layout >= 0);
        if self.used_layouts.len() > layout {
            assert!(!self.used_layouts[layout]);
            *self.used_layouts.get_mut(layout) = true;
        } else {
            self.used_layouts.grow_set(layout, &false, true);
        }
        self.attributes.push(VertexAttribute {
            layout: layout as GLuint,
            gl_type: gl_type,
            size: size as GLint,
            normalized: normalized as u8,
            offset: offset,
        });
        self
    }

    pub fn build(&self) -> VertexBuffer<VertexType> {
        VertexBuffer::new(self.attributes.clone())
    }
}

fn bind_attributes(stride: uint, attributes: &[VertexAttribute]) {
    let stride = stride as i32;
    for attr in attributes.iter() {
        check_gl!(gl::EnableVertexAttribArray(attr.layout));
        check_gl_unsafe!(gl::VertexAttribPointer(attr.layout, attr.size,
                                                 attr.gl_type, attr.normalized,
                                                 stride, attr.offset));
    }
}

fn unbind_attributes(attributes: &[VertexAttribute]) {
    for attr in attributes.iter() {
        check_gl!(gl::DisableVertexAttribArray(attr.layout));
    }
}

struct VboId {
    id: GLuint
}
impl VboId {
    fn new() -> VboId {
        let mut id : GLuint = 0;
        check_gl_unsafe!(gl::GenBuffers(1, &mut id));
        assert!(id != 0);
        VboId { id: id }
    }

    fn orphan(&mut self) -> &mut VboId {
        if self.id != 0 {
            check_gl_unsafe!(gl::DeleteBuffers(1, &self.id));
            self.id = 0;
        }
        self
    }

    fn reset(&mut self) -> &mut VboId {
        self.orphan();
        check_gl_unsafe!(gl::GenBuffers(1, &mut self.id));
        assert!(self.id != 0);
        self
    }

    fn id(&self) -> GLuint { self.id }
}
impl Drop for VboId {
    fn drop(&mut self) {
        self.orphan();
    }
}

pub struct VertexBuffer<VertexType: Copy> {
    id: VboId,
    length: uint,
    attributes: Vec<VertexAttribute>,
}

impl<VertexType: Copy> VertexBuffer<VertexType> {
    fn new(attributes: Vec<VertexAttribute>) -> VertexBuffer<VertexType> {
        VertexBuffer {
            id: VboId::new(),
            length: 0,
            attributes: attributes,
        }
    }

    pub fn draw_triangles(&self) -> &VertexBuffer<VertexType> {
        check_gl!(gl::BindBuffer(gl::ARRAY_BUFFER, self.id.id()));
        bind_attributes(mem::size_of::<VertexType>(),
                        self.attributes.as_slice());
        check_gl!(gl::DrawArrays(gl::TRIANGLES, 0, self.length as i32));
        unbind_attributes(self.attributes.as_slice());
        check_gl!(gl::BindBuffer(gl::ARRAY_BUFFER, 0));
        self
    }

    pub fn len(&self) -> uint { self.length }

    pub fn set_data(&mut self, usage: GLenum, data: &[VertexType])
            -> &mut VertexBuffer<VertexType> {
        check_gl!(gl::BindBuffer(gl::ARRAY_BUFFER, self.id.id()));
        self.length = data.len();
        check_gl_unsafe!(gl::BufferData(
                gl::ARRAY_BUFFER,
                (data.len() * mem::size_of::<VertexType>()) as GLsizeiptr,
                data.as_ptr() as *const libc::c_void, usage));
        check_gl!(gl::BindBuffer(gl::ARRAY_BUFFER, 0));
        self
    }
}
