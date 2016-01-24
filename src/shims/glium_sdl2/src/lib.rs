extern crate glium;
extern crate sdl2;

use sdl2::video::WindowBuilder;
use sdl2::ErrorMessage;
use glium::{Frame, GliumCreationError};

pub struct SDL2Facade;

impl SDL2Facade {
    pub fn draw(&self) -> Frame {
        Frame
    }
}

pub trait DisplayBuild: Sized {
    fn build_glium(self) -> Result<SDL2Facade, GliumCreationError<ErrorMessage>> {
        Ok(SDL2Facade)
    }
}

impl DisplayBuild for WindowBuilder {}
