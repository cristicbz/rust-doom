#![feature(tuple_indexing)]
#![feature(macro_rules)]
#![feature(phase)]
#![feature(globs)]

#[phase(plugin, link)]
extern crate log;
extern crate sdl2;
extern crate serialize;
extern crate gl;
extern crate libc;
extern crate native;
extern crate time;
extern crate getopts;


use ctrl::GameController;
use getopts::{optopt,optflag,getopts, usage};
use level::Level;
use libc::c_void;
use mat4::Mat4;
use numvec::Vec3;
use player::Player;
use sdl2::scancode;
use std::default::Default;
use std::os;
use wad::TextureDirectory;


#[macro_escape]
pub mod check_gl;
pub mod camera;
pub mod ctrl;
pub mod mat4;
pub mod numvec;
pub mod player;
pub mod shader;
pub mod wad;
pub mod level;
pub mod vbo;
pub mod line;
pub mod texture;
pub mod render;


static WINDOW_TITLE: &'static str = "Rusty Doom v0.0.6";
static OPENGL_MAJOR_VERISON: int = 3;
static OPENGL_MINOR_VERISON: int = 3;
static OPENGL_DEPTH_SIZE: int = 24;


pub struct MainWindow {
    window: sdl2::video::Window,
    _context: sdl2::video::GLContext,
}
impl MainWindow {
    pub fn new(width: uint, height: uint) -> MainWindow {
        sdl2::video::gl_set_attribute(sdl2::video::GLContextMajorVersion,
                                      OPENGL_MAJOR_VERISON);
        sdl2::video::gl_set_attribute(sdl2::video::GLContextMinorVersion,
                                      OPENGL_MINOR_VERISON);
        sdl2::video::gl_set_attribute(sdl2::video::GLDepthSize,
                                      OPENGL_DEPTH_SIZE);
        sdl2::video::gl_set_attribute(sdl2::video::GLDoubleBuffer, 1);

        let window = sdl2::video::Window::new(
            WINDOW_TITLE, sdl2::video::PosCentered, sdl2::video::PosCentered,
            width as int, height as int,
            sdl2::video::OpenGL | sdl2::video::Shown).unwrap();

        let context = window.gl_create_context().unwrap();
        sdl2::clear_error();
        gl::load_with(|name| {
            match sdl2::video::gl_get_proc_address(name) {
                Some(glproc) => glproc as *const libc::c_void,
                None => {
                    warn!("missing GL function: {}", name);
                    std::ptr::null()
                }
            }
        });
        unsafe {
            let mut vao_id = 0;
            check_gl!(gl::GenVertexArrays(1, &mut vao_id));
            check_gl!(gl::BindVertexArray(vao_id));
        }
        MainWindow {
           window: window,
            _context: context,
        }
    }

    pub fn aspect_ratio(&self) -> f32 {
        let (w, h) = self.window.get_size();
        w as f32 / h as f32
    }

    pub fn swap_buffers(&self) {
        self.window.gl_swap_window();
    }
}

struct GameConfig<'a> {
    wad: &'a str,
    level_index: uint,
    fov: f32,
}

struct Game {
    window: MainWindow,
    player: Player,
    level: Level,
}
impl Game {
    fn new<'a>(window: MainWindow, config: GameConfig<'a>) -> Game {
        let mut wad = wad::Archive::open(&Path::new(config.wad)).unwrap();
        let textures = TextureDirectory::from_archive(&mut wad).unwrap();
        let level_name = *wad.get_level_name(config.level_index);
        let level = Level::new(&mut wad, &textures, &level_name);

        check_gl!(gl::ClearColor(0.06, 0.07, 0.09, 0.0));
        check_gl!(gl::Enable(gl::DEPTH_TEST));
        check_gl!(gl::DepthFunc(gl::LESS));

        let start = *level.get_start_pos();
        let mut player = Player::new(config.fov, window.aspect_ratio(),
                                     Default::default());
        player.set_position(&Vec3::new(start.x, 0.3, start.y));

        Game {
            window: window,
            player: player,
            level: level
        }
    }

    fn run(&mut self) {
        let quit_gesture = ctrl::AnyGesture(
            vec![ctrl::QuitTrigger,
                 ctrl::KeyTrigger(scancode::EscapeScanCode)]);

        let mut cum_time = 0.0;
        let mut cum_updates_time = 0.0;
        let mut num_frames = 0.0;
        let mut t0 = 0.0;
        let mut control = GameController::new();
        loop {
            check_gl!(gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT));
            let t1 = time::precise_time_s();
            let mut delta = (t1 - t0) as f32;
            if delta < 1e-10 { delta = 1.0 / 60.0; }
            let delta = delta;
            t0 = t1;

            let updates_t0 = time::precise_time_s();

            control.update();
            if control.poll_gesture(&quit_gesture) {
                break;
            }
            self.player.update(delta, &control);
            self.level.render(
                delta,
                &self.player.get_camera()
                .multiply_transform(&Mat4::new_identity()));

            let updates_t1 = time::precise_time_s();
            cum_updates_time += updates_t1 - updates_t0;

            cum_time += delta as f64;
            num_frames += 1.0 as f64;
            if cum_time > 2.0 {
                let fps = num_frames / cum_time;
                let cpums = 1000.0 * cum_updates_time / num_frames as f64;
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


fn main() {
    let args: Vec<String> = os::args();
    let opts = [
        optopt("i", "iwad",
               "set initial wad file to use wad [default='doom1.wad']", "FILE"),
        optopt("l", "level",
               "the index of the level to render [default=0]", "N"),
        optopt("f", "fov",
               "horizontal field of view to please TotalHalibut [default=65]",
               "FLOAT"),
        optopt("r", "resolution",
               "the resolution at which to render the game [default=1280x720]",
               "WIDTHxHEIGHT"),
        optflag("d", "dump-levels", "list all levels and exit."),
        optflag("", "load-all", "loads all levels and exit; for debugging"),
        optflag("h", "help", "print this help message and exit"),
    ];

    let matches = match getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(f) => fail!(f.to_string()),
    };

    let wad_filename = matches
        .opt_str("i")
        .unwrap_or("doom1.wad".to_string());
    let (width, height) = matches
        .opt_str("r")
        .map(|r| {
            let v = r.as_slice().splitn(1, 'x').collect::<Vec<&str>>();
            if v.len() != 2 { None } else { Some(v) }
            .and_then(|v| from_str::<uint>(v[0]).map(|v0| (v0, v[1])))
            .and_then(|(v0, s)| from_str::<uint>(s).map(|v1| (v0, v1)))
            .expect("Invalid format for resolution, please use WIDTHxHEIGHT.")
        })
        .unwrap_or((1280, 720));
    let level_index = matches
        .opt_str("l")
        .map(|l| from_str::<uint>(l.as_slice())
            .expect("Invalid value for --level. Expected integer."))
        .unwrap_or(0);
    let fov = matches
        .opt_str("f")
        .map(|f| from_str::<f32>(f.as_slice())
             .expect("Invalid value for --fov. Expected float."))
        .unwrap_or(65.0);

    if matches.opt_present("h") {
        println!("{}", usage("A rust doom renderer.", &opts));
        return;
    }

    if matches.opt_present("d") {
        let wad = wad::Archive::open(&Path::new(wad_filename)).unwrap();
        for i_level in range(0, wad.num_levels()) {
            println!("{:3} {:8}", i_level, wad.get_level_name(i_level));
        }
        return;
    }

    if matches.opt_present("load-all") {
        if !sdl2::init(sdl2::InitVideo) {
            fail!("main: sdl video init failed.");
        }
        let _win = MainWindow::new(width, height);
        let t0 = time::precise_time_s();
        let mut wad = wad::Archive::open(
            &Path::new(wad_filename.as_slice())).unwrap();
        let textures = TextureDirectory::from_archive(&mut wad).unwrap();
        for level_index in range(0, wad.num_levels()) {
            let level_name = *wad.get_level_name(level_index);
            let level = Level::new(&mut wad, &textures, &level_name);
        }
        println!("Done, loaded all levels in {:.4}s. Shutting down...",
                 time::precise_time_s() - t0);
        sdl2::quit();
        return;
    }

    if !sdl2::init(sdl2::InitVideo) { fail!("main: sdl video init failed."); }

    let mut game = Game::new(
        MainWindow::new(width, height),
        GameConfig {
            wad: wad_filename.as_slice(),
            level_index: level_index,
            fov: fov,
        });
    game.run();

    info!("Shutting down.");
    drop(game);
    sdl2::quit();
}


//#[start]
//fn start(argc: int, argv: *const *const u8) -> int {
//    native::start(argc, argv, main)
//}

