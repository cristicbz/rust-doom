use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};

pub type Coord = i32;
pub type Size = u32;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ErrorMessage(pub String);

pub type SdlResult<T> = Result<T, ErrorMessage>;

impl Display for ErrorMessage {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        write!(fmt, "{}", self.0)
    }
}

impl Error for ErrorMessage {
    fn description(&self) -> &str {
        &self.0
    }

    fn cause(&self) -> Option<&Error> {
        None
    }
}

pub mod keyboard {
    #[derive(Copy, Clone, Debug)]
    pub enum Scancode {
        A,
        C,
        D,
        Down,
        Escape,
        F,
        Grave,
        H,
        Left,
        Right,
        S,
        Space,
        Up,
        W,
    }
}

pub mod event {
    pub enum Event {
        Quit {
            dummy: (),
        },
        KeyDown {
            scancode: Option<super::keyboard::Scancode>,
        },
        KeyUp {
            scancode: Option<super::keyboard::Scancode>,
        },
        MouseMotion {
            xrel: i32,
            yrel: i32,
        },
        OtherEvent,
    }
}
pub mod mouse {
    pub struct Mouse;
    pub struct MouseUtil;
    impl MouseUtil {
        pub fn set_relative_mouse_mode(&self, _: bool) {}
    }
}

pub struct Sdl;
pub struct EventPump;

impl Sdl {
    pub fn video(&self) -> SdlResult<video::VideoSubsystem> {
        Ok(video::VideoSubsystem)
    }

    pub fn event_pump(&self) -> SdlResult<EventPump> {
        Ok(EventPump)
    }

    pub fn mouse(&self) -> mouse::MouseUtil {
        mouse::MouseUtil
    }
}

pub type EventPollIterator = ::std::option::IntoIter<event::Event>;

impl EventPump {
    pub fn poll_iter(&mut self) -> EventPollIterator {
        None.into_iter()
    }
}

pub fn init() -> SdlResult<Sdl> {
    Ok(Sdl)
}

pub mod version {
    pub use std::fmt::{Display, Formatter, Result as FmtResult};

    pub struct Version(pub String);
    impl Display for Version {
        fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
            write!(fmt, "{}", self.0)
        }
    }
}

pub mod video {
    pub use super::*;

    pub enum GLProfile {
        Core,
    }

    pub struct GLAttr;

    impl GLAttr {
        pub fn set_context_profile(&self, _profile: GLProfile) {}
        pub fn set_context_major_version(&self, _v: u8) {}
        pub fn set_context_minor_version(&self, _v: u8) {}
        pub fn set_depth_size(&self, _v: u8) {}
        pub fn set_double_buffer(&self, _v: bool) {}
    }

    pub struct VideoSubsystem;

    impl VideoSubsystem {
        pub fn gl_attr(&self) -> GLAttr {
            GLAttr
        }

        pub fn window(&self, _title: &str, _width: Size, _height: Size) -> WindowBuilder {
            WindowBuilder
        }
    }

    pub struct WindowBuilder;
    impl WindowBuilder {
        pub fn position_centered(self) -> Self {
            self
        }
        pub fn opengl(self) -> Self {
            self
        }
        pub fn resizable(self) -> Self {
            self
        }
    }
}

pub fn clear_error() {}


pub mod pixels {
    pub enum Color {
        RGBA(u8, u8, u8, u8),
    }

    pub enum PixelFormatEnum {
        ARGB8888,
        BGR24,
    }
}

pub mod rect {
    use super::*;

    #[allow(unused)]
    pub struct Rect {
        x: Coord,
        y: Coord,
        width: Size,
        height: Size,
    }

    impl Rect {
        pub fn new_unwrap(x: Coord, y: Coord, width: Size, height: Size) -> Self {
            Rect {
                x: x,
                y: y,
                width: width,
                height: height,
            }
        }
    }
}

pub mod render {
    pub enum BlendMode {
        None,
        Blend,
    }
}

pub mod surface {
    use super::*;
    use std::marker::PhantomData;

    pub struct Surface<'a> {
        width: Size,
        height: Size,
        _phantom: PhantomData<&'a ()>,
    }

    impl<'a> Surface<'a> {
        pub fn new(width: Size,
                   height: Size,
                   _format: super::pixels::PixelFormatEnum)
                   -> SdlResult<Self> {
            Ok(Surface {
                width: width,
                height: height,
                _phantom: PhantomData,
            })
        }

        pub fn from_data(_data: &mut [u8],
                         width: Size,
                         height: Size,
                         _pitch: Size,
                         _format: super::pixels::PixelFormatEnum)
                         -> SdlResult<Surface> {
            Ok(Surface {
                width: width,
                height: height,
                _phantom: PhantomData,
            })
        }

        pub fn set_blend_mode(&mut self, _blend: super::render::BlendMode) -> SdlResult<()> {
            Ok(())
        }

        pub fn fill_rect(&mut self,
                         _rect: Option<super::rect::Rect>,
                         _color: super::pixels::Color)
                         -> SdlResult<()> {
            Ok(())
        }

        pub fn with_lock<R, F: FnOnce(&[u8]) -> R>(&self, fun: F) -> R {
            fun(&[])
        }

        pub fn width(&self) -> Size {
            self.width
        }

        pub fn height(&self) -> Size {
            self.height
        }

        pub fn blit(&self,
                    _src_rect: Option<super::rect::Rect>,
                    _dest: &mut Surface,
                    _dest_rect: Option<super::rect::Rect>)
                    -> SdlResult<()> {
            Ok(())
        }

        pub fn save_bmp<P>(&self, _path: P) -> SdlResult<()> {
            Ok(())
        }
    }
}
