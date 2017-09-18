use super::SHADER_ROOT;
use super::errors::Result;
use super::level::Level;
use super::player::Player;
use engine::{Input, Gesture, Scene, SceneBuilder, Window, Scancode, Camera};
use engine::TextRenderer;
use math::Vec2f;
use std::path::PathBuf;
use time;
use wad::{Archive, TextureDirectory};

pub struct GameConfig {
    pub wad_file: PathBuf,
    pub metadata_file: PathBuf,
    pub level_index: usize,
    pub fov: f32,
    pub width: u32,
    pub height: u32,
    pub version: &'static str,
}


pub struct Game {
    window: Window,
    scene: Scene,
    text: TextRenderer,
    player: Player,
    camera: Camera,
    level: Level,
    input: Input,
}

impl Game {
    pub fn new(config: GameConfig) -> Result<Game> {
        let window = Window::new(
            config.width,
            config.height,
            &format!("Rusty Doom v{}", config.version),
        )?;
        let wad = Archive::open(&config.wad_file, &config.metadata_file)?;
        ensure!(
            config.level_index < wad.num_levels(),
            "Level index was {}, must be between 0..{}, run with --list-levels to see names.",
            config.level_index,
            wad.num_levels() - 1
        );
        let textures = TextureDirectory::from_archive(&wad)?;
        let (level, scene) = {
            let mut scene = SceneBuilder::new(&window, PathBuf::from(SHADER_ROOT));
            let level = Level::new(&wad, &textures, config.level_index, &mut scene)?;
            let scene = scene.build()?;
            (level, scene)
        };

        let mut camera = Camera::new(config.fov, window.aspect_ratio() * 1.2, NEAR, FAR);
        let mut player = Player::new(Default::default());
        player.setup(&mut camera, level.start_pos(), level.start_yaw());

        let input = Input::new(&window)?;
        let text = TextRenderer::new(&window)?;

        Ok(Game {
            window,
            player,
            camera,
            level,
            scene,
            text,
            input,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        let Game {
            ref mut window,
            ref mut player,
            ref mut camera,
            ref mut level,
            ref mut scene,
            ref mut text,
            ref mut input,
        } = *self;

        let quit_gesture = Gesture::AnyOf(vec![
            Gesture::QuitTrigger,
            Gesture::KeyTrigger(Scancode::Escape),
        ]);
        let grab_toggle_gesture = Gesture::KeyTrigger(Scancode::Grave);
        let help_gesture = Gesture::KeyTrigger(Scancode::H);

        let short_help = text.insert(window, SHORT_HELP, Vec2f::new(0.0, 0.0), 6);
        let long_help = text.insert(window, LONG_HELP, Vec2f::new(0.0, 0.0), 6);
        text[long_help].set_visible(false);
        let mut current_help = 0;

        let mut cum_time = 0.0;
        let mut cum_updates_time = 0.0;
        let mut num_frames = 0.0;
        let mut t0 = time::precise_time_s();
        let mut mouse_grabbed = true;
        let mut running = true;
        input.set_mouse_enabled(true);
        input.set_cursor_grabbed(true);
        while running {
            let t1 = time::precise_time_s();
            let mut delta = (t1 - t0) as f32;
            if delta < 1e-10 {
                delta = 1.0 / 60.0;
            }
            let delta = delta;
            t0 = t1;

            let updates_t0 = time::precise_time_s();

            input.update();
            if input.poll_gesture(&quit_gesture) {
                running = false;
            } else if input.poll_gesture(&grab_toggle_gesture) {
                mouse_grabbed = !mouse_grabbed;
                input.set_mouse_enabled(mouse_grabbed);
                input.set_cursor_grabbed(mouse_grabbed);
            } else if input.poll_gesture(&help_gesture) {
                current_help = current_help % 2 + 1;
                match current_help {
                    0 => text[short_help].set_visible(true),
                    1 => {
                        text[short_help].set_visible(false);
                        text[long_help].set_visible(true);
                    }
                    2 => text[long_help].set_visible(false),
                    _ => unreachable!(),
                }
            }

            player.update(camera, delta, input, level);
            level.update(delta, scene);

            let mut frame = window.draw();
            scene.render(&mut frame, camera, delta)?;
            text.render(&mut frame)?;
            // TODO(cristicbz): Re-architect a little bit to support rebuilding the context.
            frame.finish().expect(
                "Cannot handle context loss currently :(",
            );

            let updates_t1 = time::precise_time_s();
            cum_updates_time += updates_t1 - updates_t0;

            cum_time += f64::from(delta);
            num_frames += 1.0;
            if cum_time > 2.0 {
                let fps = num_frames / cum_time;
                let cpums = 1000.0 * cum_updates_time / num_frames;
                info!(
                    "Frame time: {:.2}ms ({:.2}ms cpu, FPS: {:.2})",
                    1000.0 / fps,
                    cpums,
                    fps
                );
                cum_time = 0.0;
                cum_updates_time = 0.0;
                num_frames = 0.0;
            }

        }
        Ok(())
    }
}

const NEAR: f32 = 0.01;
const FAR: f32 = 100.0;

const SHORT_HELP: &'static str = "Press 'h' for help.";
const LONG_HELP: &'static str = r"Use WASD or arrow keys to move and the mouse to aim.
Other keys:
    ESC - to quit
    SPACEBAR - jump
    ` - to toggle mouse grab (backtick)
    f - to toggle fly mode
    c - to toggle clipping (wall collisions)
    h - toggle this help message";
