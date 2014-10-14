#[cfg(target_os = "linux")]
generate_gl_bindings!("gl", "core", "3.0", "global")

#[cfg(not(target_os = "linux"))]
generate_gl_bindings!("gl", "core", "3.3", "global")
