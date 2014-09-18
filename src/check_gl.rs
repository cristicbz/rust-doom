use gl;

#[cfg(not(disable_gl_checks))]
fn error_code_to_string(err: u32) -> &'static str {
    match err {
        gl::INVALID_ENUM                  => "Invalid enum",
        gl::INVALID_VALUE                 => "Invalid value",
        gl::INVALID_OPERATION             => "Invalid operation",
        gl::INVALID_FRAMEBUFFER_OPERATION => "Invalid frame buffer operation",
        gl::OUT_OF_MEMORY                 => "Out of memory",
        _                                 => "Unknown error"
    }
}

#[cfg(not(disable_gl_checks))]
pub fn detail_check_gl(
        filename      : &'static str,
        line          : uint,
        function_call : &'static str) {
    let gl_error_code = gl::GetError();
    if gl_error_code != gl::NO_ERROR {
        fail!("OpenGL Error in call '{}' at {}:{}; error code: {} ({}).",
              function_call, filename, line,
              gl_error_code, error_code_to_string(gl_error_code));
    }
}

#[cfg(disable_gl_checks)]
pub fn detail_check_gl(
        _filename      : &'static str,
        _line          : uint,
        _function_call : &'static str) {}

macro_rules! check_gl (
  ($func:expr) => ({
    let ret = $func;
    check_gl::detail_check_gl(file!(), line!(), stringify!($func));
    ret
  });
)

pub macro_rules! check_gl_unsafe (
  ($func:expr) => ({
    unsafe { check_gl!($func) }
  });
)
