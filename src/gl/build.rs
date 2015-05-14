extern crate gl_generator;
extern crate khronos_api;

use std::env;
use std::fs::File;
use std::path::PathBuf;

fn main() {
    let dest = PathBuf::from(&env::var("OUT_DIR").unwrap())
        .join("gl_bindings.rs");
    let mut file = File::create(&dest).unwrap();

    let version = if cfg!(target_os = "linux") { "3.0" } else { "3.3" };

    gl_generator::generate_bindings(gl_generator::GlobalGenerator,
                                    gl_generator::registry::Ns::Gl,
                                    gl_generator::registry::Fallbacks::None,
                                    khronos_api::GL_XML,
                                    vec![],
                                    version,
                                    "core",
                                    &mut file).unwrap();
}
