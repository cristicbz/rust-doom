#[macro_use] extern crate log;
#[macro_use] extern crate gl;

extern crate gfx;
extern crate wad;
extern crate math;

extern crate num;
extern crate getopts;
extern crate libc;

extern crate env_logger;
extern crate sdl2;
extern crate time;

use ctrl::GameController;
use ctrl::Gesture;
use gfx::ShaderLoader;
use level::Level;
use libc::c_void;
use math::Vec3;
use player::Player;
use sdl2::keyboard::Scancode;
use sdl2::Sdl;
use sdl2::video::{gl_attr, GLProfile};
use std::default::Default;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fmt::Result as FmtResult;
use std::path::PathBuf;
use wad::{Archive, TextureDirectory};

pub mod camera;
pub mod ctrl;
pub mod player;
pub mod level;
pub mod cached;
pub mod lights;


const WINDOW_TITLE: &'static str =
    "Rusty Doom v0.0.7 - Toggle mouse with backtick key (`))";
const OPENGL_DEPTH_SIZE: u8 = 24;
const SHADER_ROOT: &'static str = "src/shaders";


#[derive(Debug)]
pub struct GeneralError(String);
impl Error for GeneralError {
    fn description(&self) -> &str { &self.0[..] }
}
impl From<String> for GeneralError {
    fn from(message: String) -> GeneralError { GeneralError(message) }
}
impl<'a> From<&'a str> for GeneralError {
    fn from(message: &'a str) -> GeneralError { GeneralError(message.to_owned()) }
}
impl Display for GeneralError {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        write!(fmt, "{}", self.0)
    }
}

pub struct MainWindow {
    window: sdl2::video::Window,
    width: usize,
    height: usize,
    _context: sdl2::video::GLContext,
}
impl MainWindow {
    pub fn new(sdl: &Sdl, width: usize, height: usize) -> Result<MainWindow, Box<Error>> {
        gl_attr::set_context_profile(GLProfile::Core);
        gl_attr::set_context_major_version(gl::platform::GL_MAJOR_VERSION);
        gl_attr::set_context_minor_version(gl::platform::GL_MINOR_VERSION);
        gl_attr::set_depth_size(OPENGL_DEPTH_SIZE);
        gl_attr::set_double_buffer(true);

        let window = try!(sdl.window(WINDOW_TITLE, width as u32, height as u32)
            .position_centered()
            .opengl()
            .build()
            .map_err(GeneralError));

        let context = try!(window.gl_create_context().map_err(GeneralError));
        sdl2::clear_error();
        gl::load_with(|name| {
            sdl2::video::gl_get_proc_address(name) as *const libc::c_void
        });
        let mut vao_id = 0;
        check_gl_unsafe!(gl::GenVertexArrays(1, &mut vao_id));
        check_gl_unsafe!(gl::BindVertexArray(vao_id));
        Ok(MainWindow {
           window: window,
           width: width,
           height: height,
           _context: context,
        })
    }

    pub fn aspect_ratio(&self) -> f32 {
        self.width as f32 / self.height as f32
    }

    pub fn swap_buffers(&self) {
        self.window.gl_swap_window();
    }
}

pub struct GameConfig<'a> {
    wad: &'a str,
    metadata: &'a str,
    level_index: usize,
    fov: f32,
}

pub struct Game {
    window: MainWindow,
    player: Player,
    level: Level,
}
impl Game {
    pub fn new(window: MainWindow, config: GameConfig) -> Result<Game, Box<Error>> {
        let mut wad = try!(Archive::open(&config.wad, &config.metadata));
        let textures = try!(TextureDirectory::from_archive(&mut wad));
        let shader_loader = ShaderLoader::new(gl::platform::GLSL_VERSION_STRING,
                                              PathBuf::from(SHADER_ROOT));
        let level = try!(Level::new(&shader_loader, &mut wad, &textures, config.level_index));

        check_gl_unsafe!(gl::ClearColor(0.06, 0.07, 0.09, 0.0));
        check_gl_unsafe!(gl::Enable(gl::DEPTH_TEST));
        check_gl_unsafe!(gl::DepthFunc(gl::LESS));

        let start = *level.start_pos();
        let mut player = Player::new(config.fov, window.aspect_ratio(), Default::default());
        player.set_position(&Vec3::new(start.x, 0.3, start.y));

        Ok(Game {
            window: window,
            player: player,
            level: level
        })
    }

    pub fn run(&mut self, sdl: &mut Sdl) {
        let quit_gesture = Gesture::AnyOf(
            vec![Gesture::QuitTrigger,
                 Gesture::KeyTrigger(Scancode::Escape)]);
        let grab_toggle_gesture = Gesture::KeyTrigger(Scancode::Grave);

        let mut cum_time = 0.0;
        let mut cum_updates_time = 0.0;
        let mut num_frames = 0.0;
        let mut t0 = time::precise_time_s();
        let mut control = GameController::new(sdl.event_pump());
        let mut mouse_grabbed = true;
        loop {
            check_gl_unsafe!(
                gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT));
            let t1 = time::precise_time_s();
            let mut delta = (t1 - t0) as f32;
            if delta < 1e-10 { delta = 1.0 / 60.0; }
            let delta = delta;
            t0 = t1;

            let updates_t0 = time::precise_time_s();

            control.update();
            if control.poll_gesture(&quit_gesture) {
                break;
            } else if control.poll_gesture(&grab_toggle_gesture) {
                mouse_grabbed = !mouse_grabbed;
                control.set_mouse_enabled(mouse_grabbed);
                control.set_cursor_grabbed(mouse_grabbed);
            }

            self.player.update(delta, &control, &self.level);
            self.level.render(delta,
                              self.player.camera().projection(), self.player.camera().modelview());

            let updates_t1 = time::precise_time_s();
            cum_updates_time += updates_t1 - updates_t0;

            cum_time += delta as f64;
            num_frames += 1.0;
            if cum_time > 2.0 {
                let fps = num_frames / cum_time;
                let cpums = 1000.0 * cum_updates_time / num_frames;
                info!("Frame time: {:.2}ms ({:.2}ms cpu, FPS: {:.2})",
                      1000.0 / fps, cpums, fps);
                cum_time = 0.0;
                cum_updates_time = 0.0;
                num_frames = 0.0;
            }

            self.window.swap_buffers();
        }
    }
}

#[cfg(not(test))]
pub fn run() -> Result<(), Box<Error>> {
    use getopts::Options;
    use std::env;

    try!(env_logger::init());

    let args = env::args().collect::<Vec<_>>();
    let mut opts = Options::new();
    opts.optopt("i", "iwad",
                "set initial wad file to use wad [default='doom1.wad']",
                "FILE");
    opts.optopt("m", "metadata",
                "path to toml toml metadata file [default='doom.toml']",
                "FILE");
    opts.optopt("l", "level",
                "the index of the level to render [default=0]", "N");
    opts.optopt("f", "fov",
                "horizontal field of view to please TotalHalibut [default=65]",
                "FLOAT");
    opts.optopt("r", "resolution",
                "the resolution at which to render the game [default=1280x720]",
                "WIDTHxHEIGHT");
    opts.optflag("d", "dump-levels", "list all levels and exit.");
    opts.optflag("", "load-all", "loads all levels and exit; for debugging");
    opts.optflag("h", "help", "print this help message and exit");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => panic!(f.to_string()),
    };

    let wad_filename = matches
        .opt_str("i")
        .unwrap_or("doom1.wad".to_string());
    let meta_filename = matches
        .opt_str("m")
        .unwrap_or("doom.toml".to_string());
    let (width, height) = matches
        .opt_str("r")
        .map(|r| {
            let r: Vec<&str> = r.splitn(2, 'x').collect();
            match &r[..] {
                wh if wh.len() == 2 => (wh[0].parse().unwrap(), wh[1].parse().unwrap()),
                _ => {
                    panic!("Invalid format for resolution, \
                            please use WIDTHxHEIGHT.");
                }
            }
        })
        .unwrap_or((1280, 720));
    let level_index = matches
        .opt_str("l")
        .map(|l| l.parse().ok()
                  .expect("Invalid value for --level. Expected integer."))
        .unwrap_or(0);
    let fov = matches
        .opt_str("f")
        .map(|f| f.parse().ok()
                    .expect("Invalid value for --fov. Expected float."))
        .unwrap_or(65.0);

    if matches.opt_present("h") {
        println!("{}", opts.usage("A rust doom renderer."));
        return Ok(());
    }

    if matches.opt_present("d") {
        let wad = try!(Archive::open(&wad_filename, &meta_filename));
        for i_level in 0..wad.num_levels() {
            println!("{:3} {:8}", i_level, wad.level_name(i_level));
        }
        return Ok(());
    }

    let mut sdl = try!(sdl2::init().video().build().map_err(GeneralError));
    let win = try!(MainWindow::new(&sdl, width, height));

    if matches.opt_present("load-all") {
        let t0 = time::precise_time_s();
        let mut wad = try!(Archive::open(&wad_filename, &meta_filename));
        let textures = try!(TextureDirectory::from_archive(&mut wad));
        let shader_loader = ShaderLoader::new(
            gl::platform::GLSL_VERSION_STRING, PathBuf::from(SHADER_ROOT));
        for level_index in 0 .. wad.num_levels() {
            try!(Level::new(&shader_loader, &mut wad, &textures, level_index));
        }
        println!("Done, loaded all levels in {:.4}s. Shutting down...",
                 time::precise_time_s() - t0);
        drop(win);
        return Ok(());
    }

    let mut game = try!(Game::new(
        win,
        GameConfig {
            wad: &wad_filename,
            metadata: &meta_filename,
            level_index: level_index,
            fov: fov,
        }));
    game.run(&mut sdl);

    info!("Shutting down.");
    Ok(())
}
