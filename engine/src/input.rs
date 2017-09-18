use super::errors::{Result, ErrorKind};
use super::window::Window;
use math::Vec2f;
use num::Zero;
use sdl2::EventPump;
use sdl2::event::Event;
pub use sdl2::keyboard::Scancode;
pub use sdl2::mouse::MouseButton;
use sdl2::mouse::MouseUtil;
use std::vec::Vec;

pub type Sensitivity = f32;

pub enum Gesture {
    NoGesture,
    KeyHold(Scancode),
    KeyTrigger(Scancode),
    ButtonHold(MouseButton),
    ButtonTrigger(MouseButton),
    AnyOf(Vec<Gesture>),
    AllOf(Vec<Gesture>),
    QuitTrigger,
}

pub enum Analog2d {
    NoAnalog2d,

    Mouse { sensitivity: Sensitivity },

    Gestures {
        x_positive: Gesture,
        x_negative: Gesture,
        y_positive: Gesture,
        y_negative: Gesture,
        step: Sensitivity,
    },

    Sum { analogs: Vec<Analog2d> },
}

pub struct Input {
    current_update_index: UpdateIndex,

    keyboard_state: [ButtonState; NUM_SCAN_CODES],
    mouse_button_state: [ButtonState; NUM_MOUSE_BUTTONS],
    quit_requested_index: UpdateIndex,

    mouse_enabled: bool,
    mouse_rel: Vec2f,
    mouse_util: MouseUtil,

    pump: EventPump,
}

impl Input {
    pub fn new(window: &Window) -> Result<Input> {
        let pump = window.sdl().event_pump().map_err(ErrorKind::Sdl)?;
        let mouse_util = window.sdl().mouse();
        Ok(Input {
            current_update_index: 1,
            keyboard_state: [ButtonState::Up(0); NUM_SCAN_CODES],
            mouse_button_state: [ButtonState::Up(0); NUM_MOUSE_BUTTONS],
            quit_requested_index: 0,
            mouse_util: mouse_util,
            mouse_enabled: true,
            mouse_rel: Vec2f::zero(),
            pump: pump,
        })
    }

    pub fn set_cursor_grabbed(&mut self, grabbed: bool) {
        self.mouse_util.set_relative_mouse_mode(grabbed);
    }

    pub fn set_mouse_enabled(&mut self, enable: bool) {
        self.mouse_enabled = enable;
    }

    pub fn update(&mut self) {
        self.current_update_index += 1;
        self.mouse_rel = Vec2f::zero();
        for event in self.pump.poll_iter() {
            match event {
                Event::Quit { .. } => {
                    self.quit_requested_index = self.current_update_index;
                }
                Event::KeyDown { scancode: Some(scancode), .. } => {
                    self.keyboard_state[scancode as usize] =
                        ButtonState::Down(self.current_update_index);
                }
                Event::KeyUp { scancode: Some(scancode), .. } => {
                    self.keyboard_state[scancode as usize] =
                        ButtonState::Up(self.current_update_index);
                }
                Event::MouseMotion {
                    xrel: x_relative,
                    yrel: y_relative,
                    ..
                } => {
                    if self.mouse_enabled {
                        self.mouse_rel = Vec2f::new(x_relative as f32, y_relative as f32);
                    } else {
                        self.mouse_rel = Vec2f::zero();
                    }
                }
                Event::MouseButtonDown { mouse_btn, .. } => {
                    if let Some(index) = mouse_button_to_index(mouse_btn) {
                        self.mouse_button_state[index] =
                            ButtonState::Down(self.current_update_index);
                    }
                }
                Event::MouseButtonUp { mouse_btn, .. } => {
                    if let Some(index) = mouse_button_to_index(mouse_btn) {
                        self.mouse_button_state[index] = ButtonState::Up(self.current_update_index);
                    }
                }
                _ => {}
            }
        }
    }

    pub fn poll_gesture(&self, gesture: &Gesture) -> bool {
        match *gesture {
            Gesture::QuitTrigger => self.quit_requested_index == self.current_update_index,
            Gesture::KeyHold(code) => {
                match self.keyboard_state[code as usize] {
                    ButtonState::Down(_) => true,
                    ButtonState::Up(_) => false,
                }
            }
            Gesture::KeyTrigger(code) => {
                match self.keyboard_state[code as usize] {
                    ButtonState::Down(index) => self.current_update_index == index,
                    ButtonState::Up(_) => false,
                }
            }
            Gesture::ButtonHold(button) => {
                match mouse_button_to_index(button) {
                    Some(index) => {
                        match self.mouse_button_state[index] {
                            ButtonState::Down(_) => true,
                            ButtonState::Up(_) => false,
                        }
                    }
                    None => false,
                }
            }
            Gesture::ButtonTrigger(button) => {
                match mouse_button_to_index(button) {
                    Some(index) => {
                        match self.mouse_button_state[index] {
                            ButtonState::Down(index) => self.current_update_index == index,
                            ButtonState::Up(_) => false,
                        }
                    }
                    None => false,
                }
            }
            Gesture::AnyOf(ref subgestures) => {
                subgestures.iter().any(
                    |subgesture| self.poll_gesture(subgesture),
                )
            }
            Gesture::AllOf(ref subgestures) => {
                subgestures.iter().all(
                    |subgesture| self.poll_gesture(subgesture),
                )
            }
            Gesture::NoGesture => false,
        }
    }

    pub fn poll_analog2d(&self, motion: &Analog2d) -> Vec2f {
        match *motion {
            Analog2d::Sum { ref analogs } => {
                analogs
                    .iter()
                    .map(|analog| self.poll_analog2d(analog))
                    .fold(Vec2f::zero(), |x, y| x + y)
            }
            Analog2d::Mouse { sensitivity } => self.mouse_rel * sensitivity,
            Analog2d::Gestures {
                ref x_positive,
                ref x_negative,
                ref y_positive,
                ref y_negative,
                step,
            } => {
                Vec2f::new(
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
                )
            }
            Analog2d::NoAnalog2d => Vec2f::zero(),
        }
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

fn mouse_button_to_index(button: MouseButton) -> Option<usize> {
    Some(match button {
        MouseButton::Left => 0,
        MouseButton::Middle => 1,
        MouseButton::Right => 2,
        MouseButton::X1 => 3,
        MouseButton::X2 => 4,
        MouseButton::Unknown => return None,
    })
}
