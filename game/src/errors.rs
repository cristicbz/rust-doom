error_chain! {
    foreign_links {}
    errors {
        Sdl(message: String) {
            description("SDL Error.")
            display("SDL Error: {}", message)
        }
    }
    links {
        Wad(::wad::Error, ::wad::ErrorKind);
        Engine(::engine::Error, ::engine::ErrorKind);
    }
}
