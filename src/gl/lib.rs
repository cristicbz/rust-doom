#![feature(macro_rules)]
#![feature(phase)]

#[phase(plugin)]
extern crate gl_generator;

#[cfg(target_os = "linux")]
generate_gl_bindings! {
    api: "gl",
    profile: "core",
    version: "3.0",
    generator: "global",
}

#[cfg(not(target_os = "linux"))]
generate_gl_bindings! {
    api: "gl",
    profile: "core",
    version: "3.3",
    generator: "global",
}

#[cfg(target_os = "linux")]
pub mod platform {
    pub const GL_MAJOR_VERSION: int = 3;
    pub const GL_MINOR_VERSION: int = 0;
    pub const GLSL_VERSION_STRING: &'static str = "300 es";
}

#[cfg(not(target_os = "linux"))]
pub mod platform {
    pub const GLSL_VERSION_STRING: &'static str = "330 core";
    pub const GL_MAJOR_VERSION: int = 3;
    pub const GL_MINOR_VERSION: int = 3;
}

pub mod check {
    #[cfg(not(disable_gl_checks))]
    fn error_code_to_string(err: u32) -> &'static str {
        match err {
            super::INVALID_ENUM => "Invalid enum",
            super::INVALID_VALUE => "Invalid value",
            super::INVALID_OPERATION => "Invalid operation",
            super::INVALID_FRAMEBUFFER_OPERATION =>
                "Invalid frame buffer operation",
            super::OUT_OF_MEMORY => "Out of memory",
            _ => "Unknown error"
        }
    }

    #[cfg(not(disable_gl_checks))]
    pub fn check_gl_helper(filename: &'static str,
                           line: uint,
                           function_call: &'static str) {
        let gl_error_code = super::GetError();
        if gl_error_code != super::NO_ERROR {
            panic!("OpenGL Error in call '{}' at {}:{}; error code: {} ({}).",
                  function_call, filename, line,
                  gl_error_code, error_code_to_string(gl_error_code));
        }
    }

    #[cfg(disable_gl_checks)]
    pub fn check_gl_helper(_filename: &'static str,
                           _line: uint,
                           _function_call: &'static str) {}
}


#[macro_export]
macro_rules! check_gl (
  ($func:expr) => ({
    let ret = $func;
    gl::check::check_gl_helper(file!(), line!(), stringify!($func));
    ret
  });
)

#[macro_export]
pub macro_rules! check_gl_unsafe (
  ($func:expr) => ({ unsafe { check_gl!($func) } });
)
