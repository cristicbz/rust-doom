use super::wad_system::WadSystem;
use engine::{
    DependenciesFrom, ControlFlow, Gesture, InfallibleSystem, Input, Scancode, TextId,
    TextRenderer, Window,
};
use math::prelude::*;
use math::Pnt2f;

pub struct Bindings {
    pub quit: Gesture,
    pub next_level: Gesture,
    pub previous_level: Gesture,
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
            next_level: Gesture::AllOf(vec![
                Gesture::KeyHold(Scancode::LControl),
                Gesture::KeyTrigger(Scancode::N),
            ]),
            previous_level: Gesture::AllOf(vec![
                Gesture::KeyHold(Scancode::LControl),
                Gesture::KeyTrigger(Scancode::P),
            ]),
            toggle_mouse: Gesture::KeyTrigger(Scancode::Grave),
            toggle_help: Gesture::KeyTrigger(Scancode::H),
        }
    }
}

#[derive(DependenciesFrom)]
pub struct Dependencies<'context> {
    bindings: &'context Bindings,
    window: &'context Window,
    input: &'context mut Input,
    text: &'context mut TextRenderer,
    control_flow: &'context mut ControlFlow,

    wad: &'context mut WadSystem,
}

pub struct Hud {
    mouse_grabbed: bool,
    current_help: HelpState,
    prompt_text: TextId,
    help_text: TextId,
}

impl<'context> InfallibleSystem<'context> for Hud {
    type Dependencies = Dependencies<'context>;

    fn debug_name() -> &'static str {
        "hud"
    }

    fn create(deps: Dependencies) -> Self {
        deps.input.set_mouse_enabled(true);
        deps.input.set_cursor_grabbed(true);

        let prompt_text = deps
            .text
            .insert(deps.window, PROMPT_TEXT, Pnt2f::origin(), HELP_PADDING);
        let help_text = deps
            .text
            .insert(deps.window, HELP_TEXT, Pnt2f::origin(), HELP_PADDING);
        deps.text[help_text].set_visible(false);

        Hud {
            prompt_text,
            help_text,
            mouse_grabbed: true,
            current_help: HelpState::Prompt,
        }
    }

    fn update(&mut self, deps: Dependencies) {
        let Dependencies {
            input,
            text,
            control_flow,
            bindings,
            ..
        } = deps;

        if input.poll_gesture(&bindings.quit) {
            control_flow.quit_requested = true
        }

        if input.poll_gesture(&bindings.toggle_mouse) {
            self.mouse_grabbed = !self.mouse_grabbed;
            input.set_mouse_enabled(self.mouse_grabbed);
            input.set_cursor_grabbed(self.mouse_grabbed);
        }

        if input.poll_gesture(&bindings.toggle_help) {
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

        if input.poll_gesture(&bindings.next_level) {
            let index = deps.wad.level_index();
            deps.wad.change_level(index + 1);
        } else if input.poll_gesture(&bindings.previous_level) {
            let index = deps.wad.level_index();
            if index > 0 {
                deps.wad.change_level(index - 1);
            }
        }
    }

    fn teardown(&mut self, deps: Dependencies) {
        deps.text.remove(self.help_text);
        deps.text.remove(self.prompt_text);
    }
}

enum HelpState {
    Prompt,
    Shown,
    Hidden,
}

const HELP_PADDING: u32 = 6;
const PROMPT_TEXT: &str = "WASD and mouse, 'E' to push/use, LB to shoot or 'h' for help.";
const HELP_TEXT: &str = r"Use WASD to move and the mouse or arrow keys to aim.
Other keys:
    ESC - to quit
    SPACEBAR - jump
    E - push/interact/use
    Left Click - shoot (only effect is to trigger gun-activated things)
    ` - to toggle mouse grab (backtick)
    f - to toggle fly mode
    c - to toggle clipping (wall collisions)
    Ctrl-N - to change to next level (though using the exit will also do this!)
    Ctrl-P - to change to previous level
    h - toggle this help message";
