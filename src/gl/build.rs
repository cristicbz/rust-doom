#![allow(unstable)]

extern crate gl_generator;
extern crate khronos_api;

use std::os;
use std::io::File;

fn main() {
    let dest = Path::new(os::getenv("OUT_DIR").unwrap());
    let mut file = File::create(&dest.join("gl_bindings.rs")).unwrap();

    let version = if cfg!(target_os = "linux") { "3.0" } else { "3.3" };

    gl_generator::generate_bindings(gl_generator::GlobalGenerator,
                                    gl_generator::registry::Ns::Gl,
                                    khronos_api::GL_XML,
                                    vec![],
                                    version,
                                    "core",
                                    &mut file).unwrap();
}
