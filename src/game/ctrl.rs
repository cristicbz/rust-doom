use math::Vec2f;
use num::Zero;
use sdl2::event::Event;
use sdl2::keyboard::Scancode;
use sdl2::mouse::{Mouse, MouseUtil};
use sdl2::{EventPump, Sdl};
use std::vec::Vec;

pub type Sensitivity = f32;

pub enum Gesture {
    NoGesture,
    KeyHold(Scancode),
    KeyTrigger(Scancode),
    ButtonHold(Mouse),
    ButtonTrigger(Mouse),
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

pub struct GameController {
    current_update_index: UpdateIndex,

    keyboard_state: [ButtonState; NUM_SCAN_CODES],
    quit_requested_index: UpdateIndex,

    mouse_enabled: bool,
    mouse_rel: Vec2f,
    mouse_util: MouseUtil,

    pump: EventPump,
}

impl GameController {
    pub fn new(sdl: &Sdl, pump: EventPump) -> GameController {
        let mouse_util = sdl.mouse();
        mouse_util.set_relative_mouse_mode(true);
        GameController {
            current_update_index: 1,
            keyboard_state: [ButtonState::Up(0); NUM_SCAN_CODES],
            quit_requested_index: 0,
            mouse_util: mouse_util,
            mouse_enabled: true,
            mouse_rel: Vec2f::zero(),
            pump: pump,
        }
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
                Event::MouseMotion { xrel: x_relative, yrel: y_relative, .. } => {
                    if self.mouse_enabled {
                        self.mouse_rel = Vec2f::new(x_relative as f32, y_relative as f32);
                    } else {
                        self.mouse_rel = Vec2f::zero();
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
                    _ => false,
                }
            }
            Gesture::KeyTrigger(code) => {
                match self.keyboard_state[code as usize] {
                    ButtonState::Down(index) => self.current_update_index == index,
                    _ => false,
                }
            }
            Gesture::AnyOf(ref subs) => {
                for subgesture in subs.iter() {
                    if self.poll_gesture(subgesture) {
                        return true;
                    }
                }
                false
            }
            Gesture::AllOf(ref subs) => {
                for subgesture in subs.iter() {
                    if !self.poll_gesture(subgesture) {
                        return false;
                    }
                }
                true
            }
            Gesture::NoGesture => false,
            _ => panic!("Unimplemented gesture type."),
        }
    }

    pub fn poll_analog2d(&self, motion: &Analog2d) -> Vec2f {
        match *motion {
            Analog2d::Sum { ref analogs } => {
                analogs.iter()
                       .map(|analog| self.poll_analog2d(analog))
                       .fold(Vec2f::zero(), |x, y| x + y)
            }
            Analog2d::Mouse { sensitivity } => self.mouse_rel * sensitivity,
            Analog2d::Gestures { ref x_positive,
                                 ref x_negative,
                                 ref y_positive,
                                 ref y_negative,
                                 step } => {
                Vec2f::new(if self.poll_gesture(x_positive) {
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
                           })
            }
            Analog2d::NoAnalog2d => Vec2f::zero(),
        }
    }
}

const NUM_SCAN_CODES: usize = 512;

type UpdateIndex = u32;

#[derive(Copy, Clone)]
enum ButtonState {
    Up(UpdateIndex),
    Down(UpdateIndex),
}
