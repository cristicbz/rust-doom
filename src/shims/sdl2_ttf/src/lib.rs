extern crate sdl2;

use std::path::Path;
use sdl2::surface::Surface;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::version::Version;
use sdl2::SdlResult;
use std::fmt::{Display, Formatter, Result as FmtResult};

#[must_use]
pub struct Sdl2TtfContext;

impl Drop for Sdl2TtfContext {
    fn drop(&mut self) {}
}

pub fn get_linked_version() -> Version {
    Version("ttf-shim".to_owned())
}

#[derive(Debug)]
pub enum Error {}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        "ttf-shim-error-description"
    }

    fn cause(&self) -> Option<&std::error::Error> {
        None
    }
}

impl Display for Error {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        write!(fmt, "ttf-shim-error-display")
    }
}

pub fn init() -> Result<Sdl2TtfContext, Error> {
    Ok(Sdl2TtfContext)
}

pub fn has_been_initialized() -> bool {
    true
}

#[derive(PartialEq)]
pub struct Font;

impl Drop for Font {
    fn drop(&mut self) {}
}

pub enum Text<'a> {
    Latin1(&'a [u8]),
    Utf8(&'a str),
    Char(char),
}

impl<'a> From<&'a str> for Text<'a> {
    fn from(string: &'a str) -> Text<'a> {
        Text::Utf8(string)
    }
}

impl<'a> From<&'a String> for Text<'a> {
    fn from(string: &'a String) -> Text<'a> {
        Text::Utf8(string)
    }
}

impl<'a> From<char> for Text<'a> {
    fn from(ch: char) -> Text<'a> {
        Text::Char(ch)
    }
}

impl<'a> From<&'a [u8]> for Text<'a> {
    fn from(bytes: &'a [u8]) -> Text<'a> {
        Text::Latin1(bytes)
    }
}

pub enum RenderMode {
    Solid {
        foreground: Color,
    },
    Shaded {
        foreground: Color,
        background: Color,
    },
    Blended {
        foreground: Color,
    },
    BlendedWrapped {
        foreground: Color,
        wrap_length: u32,
    },
}

pub fn solid<T>(foreground: T) -> RenderMode
    where T: Into<Color>
{
    RenderMode::Solid { foreground: foreground.into() }
}

pub fn blended<T>(foreground: T) -> RenderMode
    where T: Into<Color>
{
    RenderMode::Blended { foreground: foreground.into() }
}

pub fn blended_wrapped<T>(foreground: T, wrap_length: u32) -> RenderMode
    where T: Into<Color>
{
    RenderMode::BlendedWrapped {
        foreground: foreground.into(),
        wrap_length: wrap_length,
    }
}

pub fn shaded<T, U>(foreground: T, background: U) -> RenderMode
    where T: Into<Color>,
          U: Into<Color>
{
    RenderMode::Shaded {
        foreground: foreground.into(),
        background: background.into(),
    }
}

impl Font {
    pub fn from_file(_filename: &Path, _ptsize: i32) -> SdlResult<Font> {
        Ok(Font)
    }

    pub fn size<'a, T>(&self, _text: T) -> SdlResult<(u32, u32)>
        where T: Into<Text<'a>>
    {
        Ok((10, 20))
    }

    pub fn render<'a, T>(&self, _text: T, _mode: RenderMode) -> SdlResult<Surface>
        where T: Into<Text<'a>>
    {
        Surface::new(10, 20, PixelFormatEnum::ARGB8888)
    }
}
