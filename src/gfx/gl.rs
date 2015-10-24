pub mod types {
    pub type GLenum = u32;
    pub type GLuint = u32;
    pub type GLint = i32;
    pub type GLsizei = isize;
    pub type GLsizeiptr = isize;
    pub type GLchar = u8;
}

pub const VERTEX_SHADER: u32 = 0;
pub const FRAGMENT_SHADER: u32 = 1;
pub const TEXTURE_2D: u32 = 2;
pub const UNSIGNED_BYTE: u32 = 4;
pub const UNSIGNED_SHORT: u32 = 5;
pub const FLOAT: u32 = 6;
