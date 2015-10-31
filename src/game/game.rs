use common::GeneralError;
use ctrl::{GameController, Gesture};
use gfx::{Scene, SceneBuilder, Window};
use level::Level;
use player::Player;
use sdl2::keyboard::Scancode;
use sdl2::{self, Sdl};
use std::error::Error;
use std::path::PathBuf;
use super::SHADER_ROOT;
use time;
use wad::{Archive, TextureDirectory};

pub struct GameConfig {
    pub wad_file: PathBuf,
    pub metadata_file: PathBuf,
    pub level_index: usize,
    pub fov: f32,
    pub width: u32,
    pub height: u32,
}


pub struct Game {
    window: Window,
    scene: Scene,
    player: Player,
    level: Level,
    _sdl: Sdl,
    control: GameController,
}

impl Game {
    pub fn new(config: GameConfig) -> Result<Game, Box<Error>> {
        let sdl = try!(sdl2::init().map_err(|e| GeneralError(e.0)));
        let window = try!(Window::new(&sdl, config.width, config.height));


        let wad = try!(Archive::open(&config.wad_file, &config.metadata_file));
        let textures = try!(TextureDirectory::from_archive(&wad));
        let (level, scene) = {
            let mut scene = SceneBuilder::new(&window, PathBuf::from(SHADER_ROOT));
            let level = try!(Level::new(&wad, &textures, config.level_index, &mut scene));
            let scene = try!(scene.build());
            (level, scene)
        };

        let mut player = Player::new(config.fov, window.aspect_ratio() * 1.2, Default::default());
        player.set_position(level.start_pos());

        let control = GameController::new(&sdl,
                                          try!(sdl.event_pump().map_err(|e| GeneralError(e.0))));

        Ok(Game {
            window: window,
            player: player,
            level: level,
            scene: scene,
            _sdl: sdl,
            control: control,
        })
    }

    pub fn run(&mut self) -> Result<(), Box<Error>> {
        let quit_gesture = Gesture::AnyOf(vec![Gesture::QuitTrigger,
                                               Gesture::KeyTrigger(Scancode::Escape)]);
        let grab_toggle_gesture = Gesture::KeyTrigger(Scancode::Grave);

        let mut cum_time = 0.0;
        let mut cum_updates_time = 0.0;
        let mut num_frames = 0.0;
        let mut t0 = time::precise_time_s();
        let mut mouse_grabbed = true;
        loop {
            let t1 = time::precise_time_s();
            let mut delta = (t1 - t0) as f32;
            if delta < 1e-10 {
                delta = 1.0 / 60.0;
            }
            let delta = delta;
            t0 = t1;

            let updates_t0 = time::precise_time_s();

            self.control.update();
            if self.control.poll_gesture(&quit_gesture) {
                break;
            } else if self.control.poll_gesture(&grab_toggle_gesture) {
                mouse_grabbed = !mouse_grabbed;
                self.control.set_mouse_enabled(mouse_grabbed);
                self.control.set_cursor_grabbed(mouse_grabbed);
            }

            self.player.update(delta, &self.control, &self.level);
            self.scene.set_modelview(&self.player.camera().modelview());
            self.scene.set_projection(self.player.camera().projection());
            self.level.render(delta, &mut self.scene);

            let updates_t1 = time::precise_time_s();
            cum_updates_time += updates_t1 - updates_t0;

            cum_time += delta as f64;
            num_frames += 1.0;
            if cum_time > 2.0 {
                let fps = num_frames / cum_time;
                let cpums = 1000.0 * cum_updates_time / num_frames;
                info!("Frame time: {:.2}ms ({:.2}ms cpu, FPS: {:.2})",
                      1000.0 / fps,
                      cpums,
                      fps);
                cum_time = 0.0;
                cum_updates_time = 0.0;
                num_frames = 0.0;
            }
            try!(self.scene.render(&self.window, delta));
        }
        Ok(())
    }
}
