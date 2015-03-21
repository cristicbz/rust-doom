use gl;
use gl::types::*;
use libc;
use libc::c_void;
use std::marker::PhantomData;
use std::mem;

pub struct VertexBuffer {
    id: VboId,
    length: usize,
    attributes: Vec<VertexAttribute>,
    vertex_size: usize,
}
impl VertexBuffer {
    fn new(vertex_size: usize, attributes: Vec<VertexAttribute>)
            -> VertexBuffer {
        VertexBuffer {
            id: VboId::new_orphaned(),
            length: 0,
            vertex_size: vertex_size,
            attributes: attributes,
        }
    }

    pub fn draw_triangles(&self) -> &VertexBuffer {
        check_gl_unsafe!(gl::BindBuffer(gl::ARRAY_BUFFER, self.id.id()));
        bind_attributes(self.vertex_size, &self.attributes);
        check_gl_unsafe!(gl::DrawArrays(gl::TRIANGLES, 0, self.length as i32));
        unbind_attributes(&self.attributes);
        check_gl_unsafe!(gl::BindBuffer(gl::ARRAY_BUFFER, 0));
        self
    }

    pub fn len(&self) -> usize { self.length }

    pub fn set_data<V: Copy> (&mut self, usage: GLenum, data: &[V])
            -> &mut VertexBuffer {
        assert_eq!(self.vertex_size, mem::size_of::<V>());
        self.id.reset().bind();
        self.length = data.len();
        check_gl_unsafe!(gl::BufferData(
                gl::ARRAY_BUFFER, (data.len() * self.vertex_size) as GLsizeiptr,
                data.as_ptr() as *const libc::c_void, usage));
        self.id.unbind();
        self
    }
}

pub struct BufferBuilder<VertexType: Copy> {
    attributes: Vec<VertexAttribute>,
    used_layouts: Vec<bool>,
    vertex_size: usize,
    _phantom: PhantomData<VertexType>,
}
impl<VertexType: Copy>  BufferBuilder<VertexType> {
    pub fn new(capacity: usize) -> BufferBuilder<VertexType> {
        BufferBuilder {
            attributes: Vec::with_capacity(capacity),
            used_layouts: Vec::with_capacity(capacity),
            vertex_size: 0,
            _phantom: PhantomData,
        }
    }

    pub fn max_attribute_size_left(&self) -> usize {
        let final_size = mem::size_of::<VertexType>();
        assert!(final_size >= self.vertex_size);
        final_size - self.vertex_size
    }

    pub fn attribute_u8(&mut self, layout: usize, offset: *const c_void)
            -> &mut BufferBuilder<VertexType> {
        self.new_attribute(layout, gl::UNSIGNED_BYTE, 1, false, offset)
    }

    pub fn attribute_u16(&mut self, layout: usize, offset: *const c_void)
            -> &mut BufferBuilder<VertexType> {
        self.new_attribute(layout, gl::UNSIGNED_SHORT, 1, false, offset)
    }

    pub fn attribute_f32(&mut self, layout: usize, offset: *const c_void)
            -> &mut BufferBuilder<VertexType> {
        self.new_attribute(layout, gl::FLOAT, 1, false, offset)
    }

    pub fn attribute_vec2f(&mut self, layout: usize, offset: *const c_void)
            -> &mut BufferBuilder<VertexType> {
        self.new_attribute(layout, gl::FLOAT, 2, false, offset)
    }

    pub fn attribute_vec3f(&mut self, layout: usize, offset: *const c_void)
            -> &mut BufferBuilder<VertexType> {
        self.new_attribute(layout, gl::FLOAT, 3, false, offset)
    }

    pub fn new_attribute(&mut self, layout: usize, gl_type: GLenum,
                         size: usize, normalized: bool, offset: *const c_void)
            -> &mut BufferBuilder<VertexType> {
        let attr_size = size * match gl_type {
            gl::FLOAT => mem::size_of::<f32>(),
            gl::UNSIGNED_BYTE => mem::size_of::<u8>(),
            gl::UNSIGNED_SHORT => mem::size_of::<u16>(),
            _ => panic!("Unsupported attribute type.")
        };
        assert!(self.max_attribute_size_left() >= attr_size);
        self.vertex_size += attr_size;
        if self.used_layouts.len() > layout {
            assert!(!self.used_layouts[layout]);
            self.used_layouts[layout] = true;
        } else {
            for _ in 0..(layout - self.used_layouts.len()) {
                self.used_layouts.push(false);
            }
            self.used_layouts.push(true);
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

    pub fn build(&self) -> VertexBuffer {
        assert_eq!(self.max_attribute_size_left(), 0);
        VertexBuffer::new(self.vertex_size, self.attributes.clone())
    }
}

type IndexType = u16;

#[allow(raw_pointer_derive)]
#[derive(Clone)]
struct VertexAttribute {
    layout: GLuint,
    gl_type: GLenum,
    size: GLint,
    normalized: u8,
    offset: *const c_void
}


fn bind_attributes(stride: usize, attributes: &[VertexAttribute]) {
    let stride = stride as i32;
    for attr in attributes.iter() {
        check_gl_unsafe!(gl::EnableVertexAttribArray(attr.layout));
        match attr.gl_type {
            gl::FLOAT => check_gl_unsafe!(gl::VertexAttribPointer(
                attr.layout, attr.size, attr.gl_type, attr.normalized,
                stride, attr.offset)),
            gl::UNSIGNED_BYTE |
            gl::UNSIGNED_SHORT => check_gl_unsafe!(gl::VertexAttribIPointer(
                attr.layout, attr.size, attr.gl_type, stride, attr.offset)),
            _ => panic!("Missing attribute type from attrib ptr.")
        }
    }
}

fn unbind_attributes(attributes: &[VertexAttribute]) {
    for attr in attributes.iter() {
        check_gl_unsafe!(gl::DisableVertexAttribArray(attr.layout));
    }
}

struct VboId {
    id: GLuint
}
impl VboId {
    fn new_orphaned() -> VboId {
        VboId { id: 0 }
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

    fn bind(&self) -> &VboId {
        check_gl_unsafe!(gl::BindBuffer(gl::ARRAY_BUFFER, self.id));
        self
    }

    fn unbind(&self) -> &VboId {
        check_gl_unsafe!(gl::BindBuffer(gl::ARRAY_BUFFER, 0));
        self
    }

    fn id(&self) -> GLuint { self.id }
}

