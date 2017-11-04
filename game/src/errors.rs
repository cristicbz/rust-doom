error_chain! {
    foreign_links {}
    errors {}
    links {
        Wad(::wad::Error, ::wad::ErrorKind);
        Engine(::engine::Error, ::engine::ErrorKind);
    }
}
