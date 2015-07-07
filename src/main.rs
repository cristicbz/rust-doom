extern crate game;

#[cfg(not(test))]
fn main() {
    use std::io;
    use std::io::Write;
    use std::env;
    use std::path::Path;

    if let Err(error) = game::run() {
        writeln!(io::stderr(), "{}: {}",
                 Path::new(&env::args().next().unwrap()).file_name().unwrap().to_string_lossy(),
                 error).unwrap()
    };
}
