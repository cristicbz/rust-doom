use engine::{TextRenderer, Input, Gesture, Scancode, TextId, Window};
use math::Vec2f;
use num::Zero;

pub struct Bindings {
    pub quit: Gesture,
    pub toggle_mouse: Gesture,
    pub toggle_help: Gesture,
}

impl Default for Bindings {
    fn default() -> Self {
        Bindings {
            quit: Gesture::AnyOf(vec![
                Gesture::QuitTrigger,
                Gesture::KeyTrigger(Scancode::Escape),
            ]),
            toggle_mouse: Gesture::KeyTrigger(Scancode::Grave),
            toggle_help: Gesture::KeyTrigger(Scancode::H),
        }
    }
}

pub struct Hud {
    bindings: Bindings,
    mouse_grabbed: bool,
    quit_requested: bool,
    current_help: HelpState,
    prompt_text: TextId,
    help_text: TextId,
}

impl Hud {
    pub fn new(
        bindings: Bindings,
        window: &Window,
        input: &mut Input,
        text: &mut TextRenderer,
    ) -> Self {

        input.set_mouse_enabled(true);
        input.set_cursor_grabbed(true);

        let prompt_text = text.insert(window, PROMPT_TEXT, Vec2f::zero(), HELP_PADDING);
        let help_text = text.insert(window, HELP_TEXT, Vec2f::zero(), HELP_PADDING);

        text[help_text].set_visible(false);

        Hud {
            bindings,
            prompt_text,
            help_text,
            mouse_grabbed: true,
            quit_requested: false,
            current_help: HelpState::Prompt,
        }
    }

    pub fn quit_requested(&self) -> bool {
        self.quit_requested
    }

    pub fn update(&mut self, input: &mut Input, text: &mut TextRenderer) {
        if input.poll_gesture(&self.bindings.quit) {
            self.quit_requested = true;
        } else if input.poll_gesture(&self.bindings.toggle_mouse) {
            self.mouse_grabbed = !self.mouse_grabbed;
            input.set_mouse_enabled(self.mouse_grabbed);
            input.set_cursor_grabbed(self.mouse_grabbed);
        } else if input.poll_gesture(&self.bindings.toggle_help) {
            self.current_help = match self.current_help {
                HelpState::Prompt => {
                    text[self.prompt_text].set_visible(false);
                    text[self.help_text].set_visible(true);
                    HelpState::Shown
                }
                HelpState::Shown => {
                    text[self.help_text].set_visible(false);
                    HelpState::Hidden
                }
                HelpState::Hidden => {
                    text[self.help_text].set_visible(true);
                    HelpState::Shown
                }
            };
        }

    }
}

enum HelpState {
    Prompt,
    Shown,
    Hidden,
}

const HELP_PADDING: u32 = 6;
const PROMPT_TEXT: &'static str = "Press 'h' for help.";
const HELP_TEXT: &'static str = r"Use WASD or arrow keys to move and the mouse to aim.
Other keys:
    ESC - to quit
    SPACEBAR - jump
    ` - to toggle mouse grab (backtick)
    f - to toggle fly mode
    c - to toggle clipping (wall collisions)
    h - toggle this help message";
