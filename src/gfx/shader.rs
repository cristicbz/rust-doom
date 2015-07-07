use error::{Error, Result};
use gl;
use gl::types::{GLint, GLuint, GLchar};
use math::{Mat4, Vec2f, Vec3f};
use std::ffi::CString;
use std::fs::File;
use std::io::Read;
use std::io::Result as IoResult;
use std::path::{Path, PathBuf};
use std::ptr;
use std::result::Result as StdResult;

#[derive(Copy, Clone)]
pub struct Uniform(GLint);

pub struct ShaderLoader {
    root_path: PathBuf,
    version_directive: String,
}
impl ShaderLoader {
    pub fn new(version: &str, root_path: PathBuf) -> ShaderLoader {
        ShaderLoader {
            version_directive: format!("#version {}\n", version),
            root_path: root_path,
        }
    }

    pub fn load(&self, name: &str) -> Result<Shader> {
        debug!("Loading shader: {}", name);
        let frag_src = self.version_directive.clone() +
            &try!(read_utf8_file(&self.root_path.join(&(name.to_string() + ".frag"))))[..];
        let vert_src = self.version_directive.clone() +
            &try!(read_utf8_file(&self.root_path.join(&(name.to_string() + ".vert"))))[..];
        debug!("Shader '{}' loaded successfully", name);
        Shader::new_from_source(&vert_src, &frag_src)
    }
}

pub struct Shader {
    program: Program,
}

impl Shader {
    pub fn new_from_source(vertex_source: &str, fragment_source: &str) -> Result<Shader> {
        let vertex = try!(VertexShader::compile(&vertex_source));
        let fragment = try!(FragmentShader::compile(&fragment_source));
        let program = try!(Program::link(vertex, fragment));
        Ok(Shader { program: program })
    }

    pub fn bind(&self) -> &Shader {
        check_gl_unsafe!(gl::UseProgram(self.program.0));
        self
    }

    pub fn bind_mut(&mut self) -> &mut Shader {
        self.bind();
        self
    }

    pub fn unbind(&self) -> &Shader {
        check_gl_unsafe!(gl::UseProgram(0));
        self
    }

    pub fn uniform(&self, name: &str) -> Option<Uniform> {
        let c_str = CString::new(name.as_bytes()).unwrap().as_ptr();
        match check_gl_unsafe!(gl::GetUniformLocation(self.program.0, c_str)) {
            -1 => None,
            id => Some(Uniform(id)),
        }
    }

    pub fn expect_uniform(&self, name: &str) -> Uniform {
        self.uniform(name).expect(&format!("Expected uniform '{}'", name))
    }

    pub fn set_uniform_i32(&self, uniform: Uniform, value: i32) -> &Shader {
        check_gl_unsafe!(gl::Uniform1i(uniform.0, value));
        self
    }

    pub fn set_uniform_f32(&self, uniform: Uniform, value: f32) -> &Shader {
        check_gl_unsafe!(gl::Uniform1f(uniform.0, value));
        self
    }

    pub fn set_uniform_f32v(&self, uniform: Uniform, value: &[f32]) -> &Shader {
        check_gl_unsafe!(gl::Uniform1fv(uniform.0, value.len() as i32, value.as_ptr()));
        self
    }

    pub fn set_uniform_vec2f(&self, uniform: Uniform, value: &Vec2f)
            -> &Shader {
        check_gl_unsafe!(gl::Uniform2fv(uniform.0, 1, &value.x));
        self
    }

    pub fn set_uniform_vec3f(&self, uniform: Uniform, value: &Vec3f)
            -> &Shader {
        check_gl_unsafe!(gl::Uniform3fv(uniform.0, 1, &value.x));
        self
    }

    pub fn set_uniform_mat4(&self, uniform: Uniform, value: &Mat4)
            -> &Shader {
        check_gl_unsafe!(gl::UniformMatrix4fv(
            uniform.0, 1, 0u8, value.as_scalar_ptr()));
        self
    }
}

struct VertexShader(GLuint);
impl VertexShader {
    fn compile(source: &str) -> Result<VertexShader> {
        compile_any(gl::VERTEX_SHADER, source)
            .map(|id| VertexShader(id))
            .map_err(|log| Error::VertexCompile(log))
    }
}
impl Drop for VertexShader {
    fn drop(&mut self) { check_gl_unsafe!(gl::DeleteShader(self.0)); }
}


struct FragmentShader(GLuint);
impl FragmentShader {
    fn compile(source: &str) -> Result<FragmentShader> {
        compile_any(gl::FRAGMENT_SHADER, source)
            .map(|id| FragmentShader(id))
            .map_err(|log| Error::FragmentCompile(log))
    }
}
impl Drop for FragmentShader {
    fn drop(&mut self) { check_gl_unsafe!(gl::DeleteShader(self.0)); }
}


struct Program(GLuint);
impl Program {
    fn link(vertex: VertexShader, fragment: FragmentShader) -> Result<Program> {
        let program = Program(check_gl_unsafe!(gl::CreateProgram()));
        check_gl_unsafe!(gl::AttachShader(program.0, vertex.0));
        check_gl_unsafe!(gl::AttachShader(program.0, fragment.0));
        check_gl_unsafe!(gl::LinkProgram(program.0));
        if link_succeeded(program.0) {
            Ok(program)
        } else {
            Err(Error::Link(link_log(program.0)))
        }
    }
}
impl Drop for Program {
    fn drop(&mut self) { unsafe { gl::DeleteProgram(self.0); } }
}


fn compile_any(shader_type: u32, source: &str) -> StdResult<GLuint, String> {
    let id = check_gl_unsafe!(gl::CreateShader(shader_type));
    let source_len = source.len() as i32;
    let source = source.as_bytes().as_ptr() as *const i8;
    assert!(id != 0);
    check_gl_unsafe!(
        gl::ShaderSource(id, 1, &source, &source_len));
    check_gl_unsafe!(gl::CompileShader(id));
    if compilation_succeeded(id) {
        Ok(id)
    } else {
        let log = compilation_log(id);;
        check_gl_unsafe!(gl::DeleteShader(id));
        if shader_type == gl::VERTEX_SHADER {
            Err(format!("Vertex shader compilation failed:\n{}", log))
        } else {
            Err(format!("Fragment shader compilation failed:\n{}", log))
        }
    }
}


fn compilation_succeeded(id: GLuint) -> bool {
    let mut result: GLint = 0;
    check_gl_unsafe!(gl::GetShaderiv(id, gl::COMPILE_STATUS, &mut result));
    result != 0
}


fn compilation_log(shader_id: GLuint) -> String {
    let mut log_length = 0;
    check_gl_unsafe!(gl::GetShaderiv(shader_id, gl::INFO_LOG_LENGTH,
                                     &mut log_length));
    assert!(log_length > 0);
    let mut log_buffer = vec![0u8; log_length as usize];
    let log_buffer_ptr = log_buffer.as_mut_ptr() as *mut gl::types::GLchar;
    check_gl_unsafe!(gl::GetShaderInfoLog(
            shader_id, log_length, ptr::null_mut(), log_buffer_ptr));
    String::from_utf8(log_buffer).unwrap()
}


fn link_succeeded(id: GLuint) -> bool {
    let mut result: GLint = 0;
    check_gl_unsafe!(gl::GetProgramiv(id, gl::LINK_STATUS, &mut result));
    result != 0
}


fn link_log(shader_id: GLuint) -> String {
    let mut log_length = 0;
    check_gl_unsafe!(gl::GetProgramiv(shader_id, gl::INFO_LOG_LENGTH,
                                      &mut log_length));
    assert!(log_length > 0);
    let mut log_buffer = vec![0u8; log_length as usize];
    let log_buffer_ptr = log_buffer.as_mut_ptr() as *mut gl::types::GLchar;
    check_gl_unsafe!(gl::GetProgramInfoLog(
            shader_id, log_length, ptr::null_mut(), log_buffer_ptr));
    String::from_utf8(log_buffer).unwrap()
}

fn read_utf8_file(path: &Path) -> IoResult<String> {
    let mut result = String::new();
    try!(File::open(path)).read_to_string(&mut result).map(|_| result)
}
