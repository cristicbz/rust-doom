use super::errors::{Error, Result};
use super::system::System;
use super::window::Window;
use crate::internal_derive::DependenciesFrom;
use glium::glutin::event::{
    DeviceEvent, ElementState, Event, KeyboardInput, StartCause, VirtualKeyCode, WindowEvent,
};
use math::Vec2f;
use num_traits::Zero;
use std::vec::Vec;

pub use glium::glutin::event::{MouseButton, VirtualKeyCode as Scancode};

pub type Sensitivity = f32;

pub enum Gesture {
    NoGesture,
    KeyHold(VirtualKeyCode),
    KeyTrigger(VirtualKeyCode),
    ButtonHold(MouseButton),
    ButtonTrigger(MouseButton),
    AnyOf(Vec<Gesture>),
    AllOf(Vec<Gesture>),
    QuitTrigger,
}

pub enum Analog2d {
    NoAnalog2d,

    Mouse {
        sensitivity: Sensitivity,
    },

    Gestures {
        x_positive: Gesture,
        x_negative: Gesture,
        y_positive: Gesture,
        y_negative: Gesture,
        step: Sensitivity,
    },

    Sum {
        analogs: Vec<Analog2d>,
    },
}

impl Input {
    pub(crate) fn reset(&mut self) {
        self.current_update_index += 1;
        self.mouse_rel = Vec2f::zero();
    }

    pub(crate) fn handle_event(&mut self, event: Event<'_, ()>) -> bool {
        match event {
            Event::NewEvents(StartCause::WaitCancelled { .. }) => {}
            Event::NewEvents(StartCause::ResumeTimeReached {
                requested_resume, ..
            }) if requested_resume > std::time::Instant::now() => {}
            Event::NewEvents(_) => {
                self.new_step = true;
            }
            Event::MainEventsCleared => {
                let new_step = self.new_step;
                self.new_step = false;
                return new_step;
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                self.quit_requested_index = self.current_update_index;
            }
            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state,
                                virtual_keycode: Some(virtual_keycode),
                                ..
                            },
                        ..
                    },
                ..
            } => {
                self.keyboard_state[virtual_keycode as usize] = match state {
                    ElementState::Pressed => ButtonState::Down(self.current_update_index),
                    ElementState::Released => ButtonState::Up(self.current_update_index),
                }
            }
            Event::DeviceEvent {
                event: DeviceEvent::Motion { axis, value },
                ..
            } => {
                if self.mouse_enabled && axis < 2 {
                    self.mouse_rel[axis as usize] += value as f32;
                }
            }
            Event::DeviceEvent {
                event: DeviceEvent::Button { button, state },
                ..
            } => {
                let button = button as usize;
                if self.mouse_enabled && button < NUM_MOUSE_BUTTONS {
                    self.mouse_button_state[button] = match state {
                        ElementState::Pressed => ButtonState::Down(self.current_update_index),
                        ElementState::Released => ButtonState::Up(self.current_update_index),
                    }
                }
            }
            _ => {}
        }
        false
    }

    pub fn set_cursor_grabbed(&mut self, grabbed: bool) {
        self.new_mouse_grabbed = grabbed
    }

    pub fn set_mouse_enabled(&mut self, enable: bool) {
        self.mouse_enabled = enable;
    }

    pub fn poll_gesture(&self, gesture: &Gesture) -> bool {
        match *gesture {
            Gesture::QuitTrigger => self.quit_requested_index == self.current_update_index,
            Gesture::KeyHold(code) => match self.keyboard_state[code as usize] {
                ButtonState::Down(_) => true,
                ButtonState::Up(_) => false,
            },
            Gesture::KeyTrigger(code) => match self.keyboard_state[code as usize] {
                ButtonState::Down(index) => self.current_update_index == index,
                ButtonState::Up(_) => false,
            },
            Gesture::ButtonHold(button) => {
                match self.mouse_button_state[mouse_button_to_index(button)] {
                    ButtonState::Down(_) => true,
                    ButtonState::Up(_) => false,
                }
            }
            Gesture::ButtonTrigger(button) => {
                match self.mouse_button_state[mouse_button_to_index(button)] {
                    ButtonState::Down(index) => self.current_update_index == index,
                    ButtonState::Up(_) => false,
                }
            }
            Gesture::AnyOf(ref subgestures) => subgestures
                .iter()
                .any(|subgesture| self.poll_gesture(subgesture)),
            Gesture::AllOf(ref subgestures) => subgestures
                .iter()
                .all(|subgesture| self.poll_gesture(subgesture)),
            Gesture::NoGesture => false,
        }
    }

    pub fn poll_analog2d(&self, motion: &Analog2d) -> Vec2f {
        match *motion {
            Analog2d::Sum { ref analogs } => analogs
                .iter()
                .map(|analog| self.poll_analog2d(analog))
                .fold(Vec2f::zero(), |x, y| x + y),
            Analog2d::Mouse { sensitivity } => self.mouse_rel * sensitivity,
            Analog2d::Gestures {
                ref x_positive,
                ref x_negative,
                ref y_positive,
                ref y_negative,
                step,
            } => Vec2f::new(
                if self.poll_gesture(x_positive) {
                    step
                } else if self.poll_gesture(x_negative) {
                    -step
                } else {
                    0.0
                },
                if self.poll_gesture(y_positive) {
                    step
                } else if self.poll_gesture(y_negative) {
                    -step
                } else {
                    0.0
                },
            ),
            Analog2d::NoAnalog2d => Vec2f::zero(),
        }
    }
}

#[derive(DependenciesFrom)]
pub struct Dependencies<'context> {
    window: &'context mut Window,
}

pub struct Input {
    current_update_index: UpdateIndex,

    keyboard_state: [ButtonState; NUM_SCAN_CODES],
    mouse_button_state: [ButtonState; NUM_MOUSE_BUTTONS],
    quit_requested_index: UpdateIndex,
    new_step: bool,

    mouse_enabled: bool,
    mouse_grabbed: bool,
    new_mouse_grabbed: bool,
    mouse_rel: Vec2f,
}

impl<'context> System<'context> for Input {
    type Dependencies = Dependencies<'context>;
    type Error = Error;

    fn create(_deps: Dependencies) -> Result<Self> {
        Ok(Input {
            current_update_index: 1,
            keyboard_state: [ButtonState::Up(0); NUM_SCAN_CODES],
            mouse_button_state: [ButtonState::Up(0); NUM_MOUSE_BUTTONS],
            quit_requested_index: 0,
            new_step: false,
            mouse_enabled: true,
            new_mouse_grabbed: true,
            mouse_grabbed: false,
            mouse_rel: Vec2f::zero(),
        })
    }

    fn debug_name() -> &'static str {
        "input"
    }

    fn update(&mut self, deps: Dependencies) -> Result<()> {
        if self.new_mouse_grabbed != self.mouse_grabbed {
            self.mouse_grabbed = self.new_mouse_grabbed;
            deps.window
                .facade()
                .gl_window()
                .window()
                .set_cursor_grab(self.mouse_grabbed)
                .ok();
            deps.window
                .facade()
                .gl_window()
                .window()
                .set_cursor_visible(!self.mouse_grabbed);
        }
        if self.mouse_grabbed {
            let _ = deps.window.facade().gl_window().window();
        }
        Ok(())
    }
}

const NUM_SCAN_CODES: usize = 512;
const NUM_MOUSE_BUTTONS: usize = 256;

type UpdateIndex = u32;

#[derive(Copy, Clone)]
enum ButtonState {
    Up(UpdateIndex),
    Down(UpdateIndex),
}

fn mouse_button_to_index(button: MouseButton) -> usize {
    match button {
        MouseButton::Left => 1,
        MouseButton::Middle => 2,
        MouseButton::Right => 3,
        MouseButton::Other(index) => ((index + 4) as usize).min(NUM_MOUSE_BUTTONS - 1),
    }
}
