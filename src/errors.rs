error_chain! {
    foreign_links {
        Argument(::clap::Error);
    }
    errors {}
    links {
        Graphics(::gfx::Error, ::gfx::ErrorKind);
        Game(::game::Error, ::game::ErrorKind);
        Wad(::wad::error::Error, ::wad::error::ErrorKind);
    }
}
