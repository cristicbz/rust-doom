#[cfg(target_os = "linux")]
mod internal {
    pub const GL_MAJOR_VERSION: u8 = 3;
    pub const GL_MINOR_VERSION: u8 = 0;
    pub const GLSL_VERSION_STRING: &'static str = "300 es";
}

#[cfg(not(target_os = "linux"))]
mod internal {
    pub const GLSL_VERSION_STRING: &'static str = "330 core";
    pub const GL_MAJOR_VERSION: u8 = 3;
    pub const GL_MINOR_VERSION: u8 = 3;
}

pub use self::internal::*;
