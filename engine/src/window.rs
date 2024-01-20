use crate::ErrorKind;

use super::errors::{Error, Result};
use super::system::System;
use glium::backend::glutin::SimpleWindowBuilder;
use glium::glutin::surface::WindowSurface;
use glium::{Display, Frame, Surface};
use winit::event_loop::EventLoop;

pub struct WindowConfig {
    pub width: u32,
    pub height: u32,
    pub title: String,
}

pub struct Window {
    display: Display<WindowSurface>,
    window: winit::window::Window,
    event_loop: Option<EventLoop<()>>,
    width: u32,
    height: u32,
}

impl Window {
    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn aspect_ratio(&self) -> f32 {
        self.width as f32 / self.height as f32
    }

    pub fn draw(&self) -> Frame {
        let mut frame = self.display.draw();
        frame.clear_all_srgb((0.06, 0.07, 0.09, 0.0), 1.0, 0);
        frame
    }

    pub fn facade(&self) -> &Display<WindowSurface> {
        &self.display
    }

    pub fn window(&self) -> &winit::window::Window {
        &self.window
    }

    pub(crate) fn take_event_loop(&mut self) -> Option<EventLoop<()>> {
        self.event_loop.take()
    }
}

impl<'context> System<'context> for Window {
    type Dependencies = &'context WindowConfig;
    type Error = Error;

    fn create(config: &'context WindowConfig) -> Result<Self> {
        let events = EventLoop::new().map_err(|e| ErrorKind::CreateWindow(e.to_string()))?;

        let (window, display) = SimpleWindowBuilder::new()
            .with_inner_size(config.width, config.height)
            .with_title(&config.title)
            .build(&events);

        Ok(Window {
            display,
            window,
            event_loop: Some(events),
            width: config.width,
            height: config.height,
        })
    }

    fn debug_name() -> &'static str {
        "window"
    }
}
