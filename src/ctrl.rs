use numvec::{Vec2f, Vec2};
use sdl2;
use sdl2::mouse::Mouse;
use sdl2::scancode::ScanCode;
use std::vec::Vec;

pub type Sensitivity = f32;

pub enum Gesture {
    NoGesture,
    KeyHold(ScanCode),
    KeyTrigger(ScanCode),
    ButtonHold(Mouse),
    ButtonTrigger(Mouse),
    AnyGesture(Vec<Gesture>),
    AllGestures(Vec<Gesture>),
    QuitTrigger,
}

pub enum Analog2d {
    NoAnalog2d,

    // (mouse_sensitivity)
    MouseMotion(Sensitivity),

    // (xpos, xneg, ypos, yneg, step)
    GesturesAnalog2d(Gesture, Gesture, Gesture, Gesture, Sensitivity),
}

pub struct GameController {
    current_update_index: UpdateIndex,

    keyboard_state: [ButtonState, ..NUM_SCAN_CODES],
    quit_requested_index: UpdateIndex,

    mouse_rel: Vec2f,
}

impl GameController {
    pub fn new() -> GameController {
        sdl2::mouse::set_relative_mouse_mode(true);
        GameController {
            current_update_index: 1,
            keyboard_state: [Up(0), ..NUM_SCAN_CODES],
            quit_requested_index: 0,
            mouse_rel: Vec2::zero(),
        }
    }

    pub fn update(&mut self) {
        self.current_update_index += 1;
        self.mouse_rel = Vec2::zero();
        loop {
            match sdl2::event::poll_event() {
                sdl2::event::QuitEvent(_) => {
                    self.quit_requested_index = self.current_update_index;
                },
                sdl2::event::KeyDownEvent(_, _, _, code, _) => {
                    self.keyboard_state[code as uint] =
                        Down(self.current_update_index);
                },
                sdl2::event::KeyUpEvent(_, _, _, code, _) => {
                    self.keyboard_state[code as uint] =
                        Up(self.current_update_index);
                },
                sdl2::event::MouseMotionEvent(_, _, _, _, _, _, xrel, yrel) => {
                    self.mouse_rel = Vec2::new(xrel as f32, -yrel as f32);
                },
                sdl2::event::NoEvent => break,
                _ => {}
            }
        }
    }

    pub fn poll_gesture(&self, gesture: &Gesture) -> bool {
        match *gesture {
            QuitTrigger => {
                self.quit_requested_index == self.current_update_index
            },
            KeyHold(code) => match self.keyboard_state[code as uint] {
                Down(_) => true,
                _ => false
            },
            KeyTrigger(code) => match self.keyboard_state[code as uint] {
                Down(index) => self.current_update_index == index,
                _ => false
            },
            AnyGesture(ref subs) => {
                for subgesture in subs.iter() {
                    if self.poll_gesture(subgesture) {
                        return true;
                    }
                }
                false
            },
            AllGestures(ref subs) => {
                for subgesture in subs.iter() {
                    if !self.poll_gesture(subgesture) {
                        return false;
                    }
                }
                true
            },
            NoGesture => false,
            _ => { fail!("Unimplemented gesture type."); }
        }
    }

    pub fn poll_analog2d(&self, motion: &Analog2d) -> Vec2f {
        match *motion {
            MouseMotion(sensitivity) => self.mouse_rel * sensitivity,
            GesturesAnalog2d(ref xpos, ref xneg, ref ypos, ref yneg, step) => {
                Vec2::new(
                    if self.poll_gesture(xpos) { step }
                    else if self.poll_gesture(xneg) { -step }
                    else { 0.0 },
                    if self.poll_gesture(ypos) { step }
                    else if self.poll_gesture(yneg) { -step }
                    else { 0.0 }
                )
            }
            NoAnalog2d => Vec2::zero()
        }
    }
}

const NUM_SCAN_CODES : uint = 512;

type UpdateIndex = u32;

enum ButtonState {
    Up(UpdateIndex),
    Down(UpdateIndex),
}
