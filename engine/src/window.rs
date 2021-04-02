use super::errors::{Error, ErrorKind, Result};
use super::platform;
use super::system::System;
use glium::{
    glutin::{
        dpi::PhysicalSize, event_loop::EventLoop, window::WindowBuilder, Api, ContextBuilder,
        GlProfile, GlRequest,
    },
    Display, Frame, Surface,
};

const OPENGL_DEPTH_SIZE: u8 = 24;

pub struct WindowConfig {
    pub width: u32,
    pub height: u32,
    pub title: String,
}

pub struct Window {
    display: Display,
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

    pub fn facade(&self) -> &Display {
        &self.display
    }

    pub(crate) fn take_event_loop(&mut self) -> Option<EventLoop<()>> {
        self.event_loop.take()
    }
}

impl<'context> System<'context> for Window {
    type Dependencies = &'context WindowConfig;
    type Error = Error;

    fn create(config: &'context WindowConfig) -> Result<Self> {
        let events = EventLoop::new();

        let window = WindowBuilder::new()
            .with_inner_size(PhysicalSize {
                width: config.width,
                height: config.height,
            })
            .with_title(config.title.clone());

        let context = ContextBuilder::new()
            .with_gl_profile(GlProfile::Core)
            .with_gl(GlRequest::Specific(
                Api::OpenGl,
                (platform::GL_MAJOR_VERSION, platform::GL_MINOR_VERSION),
            ))
            .with_depth_buffer(OPENGL_DEPTH_SIZE);

        let display = Display::new(window, context, &events)
            .map_err(ErrorKind::create_window(config.width, config.height))?;

        Ok(Window {
            display,
            event_loop: Some(events),
            width: config.width,
            height: config.height,
        })
    }

    fn debug_name() -> &'static str {
        "window"
    }
}
