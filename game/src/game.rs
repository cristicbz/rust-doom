use super::SHADER_ROOT;
use super::errors::Result;
use super::level::Level;
use super::player::Player;
use engine::{Input, Scene, SceneBuilder, Window, Camera, FrameTimers};
use engine::TextRenderer;
use hud::Hud;
use std::path::PathBuf;
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
    hud: Hud,
    timers: FrameTimers,
}

impl Game {
    pub fn new(config: GameConfig) -> Result<Game> {
        let wad = Archive::open(&config.wad_file, &config.metadata_file)?;
        ensure!(
            config.level_index < wad.num_levels(),
            "Level index was {}, must be between 0..{}, run with --list-levels to see names.",
            config.level_index,
            wad.num_levels() - 1
        );

        let timers = FrameTimers::new();
        let window = Window::new(
            config.width,
            config.height,
            &format!("Rusty Doom v{}", config.version),
        )?;
        let mut text = TextRenderer::new(&window)?;
        let textures = TextureDirectory::from_archive(&wad)?;
        let (level, scene) = {
            let mut scene = SceneBuilder::new(&window, PathBuf::from(SHADER_ROOT));
            let level = Level::new(&wad, &textures, config.level_index, &mut scene)?;
            let scene = scene.build()?;
            (level, scene)
        };

        let mut input = Input::new(&window)?;
        let mut camera = Camera::new(config.fov, window.aspect_ratio() * 1.2, NEAR, FAR);
        let player = Player::new(Default::default(), &mut camera, &level);
        let hud = Hud::new(Default::default(), &window, &mut input, &mut text);

        Ok(Game {
            window,
            player,
            camera,
            level,
            scene,
            text,
            input,
            timers,
            hud,
        })
    }

    pub fn run(self) -> Result<()> {
        let Game {
            window,
            mut player,
            mut camera,
            mut level,
            mut scene,
            mut text,
            mut input,
            mut timers,
            mut hud,
        } = self;
        let frame_timer = timers.new_stopped("frame");
        while !hud.quit_requested() {
            let delta = timers.start(frame_timer).unwrap_or(1.0 / 60.0);

            input.update();
            hud.update(&mut input, &mut text);
            player.update(delta, &input, &level, &mut camera);
            level.update(delta, &mut scene);

            let mut frame = window.draw();
            scene.render(delta, &camera, &mut frame)?;
            text.render(&mut frame)?;
            // TODO(cristicbz): Re-architect a little bit to support rebuilding the context.
            frame.finish().expect(
                "Cannot handle context loss currently :(",
            );
        }
        Ok(())
    }
}

const NEAR: f32 = 0.01;
const FAR: f32 = 100.0;
