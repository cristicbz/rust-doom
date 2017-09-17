#[cfg_attr(feature = "cargo-clippy", allow(unused_doc_comment))]
error_chain! {
    foreign_links {}
    errors {
        Sdl(message: String) {
            description("SDL Error.")
            display("SDL Error: {}", message)
        }
    }
    links {
        Wad(::wad::error::Error, ::wad::error::ErrorKind);
        Graphics(::gfx::Error, ::gfx::ErrorKind);
    }
}
