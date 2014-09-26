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
use level::Level;
use libc::c_void;
use mat4::Mat4;
use player::Player;
use sdl2::scancode;
use std::default::Default;
use numvec::Vec3;
use wad::TextureDirectory;
use getopts::{optopt,optflag,getopts, usage};
use std::os;


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

fn create_opengl_window(title : &str,
                        width : int,
                        height : int) -> sdl2::video::Window {
    if !sdl2::init(sdl2::InitVideo) { fail!("main: sdl video init failed."); }
    sdl2::video::gl_set_attribute(sdl2::video::GLContextMajorVersion, 3);
    sdl2::video::gl_set_attribute(sdl2::video::GLContextMinorVersion, 3);
    sdl2::video::gl_set_attribute(sdl2::video::GLDepthSize, 24);
    sdl2::video::gl_set_attribute(sdl2::video::GLDoubleBuffer, 1);
    match sdl2::video::Window::new(
            title, sdl2::video::PosCentered, sdl2::video::PosCentered,
            width, height, sdl2::video::OpenGL | sdl2::video::Shown) {
        Ok(w) => w, Err(err) => fail!("failed to create window: {}", err)
    }
}

fn init_opengl(window : &sdl2::video::Window) -> sdl2::video::GLContext {
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

    context
}

struct Scene {
    player: Player,
    level: Level,
}

impl Scene {
    fn new(wad_filename: &str, level_index: uint) -> Scene {
        let mut wad = wad::Archive::open(&Path::new(wad_filename)).unwrap();
        let textures = TextureDirectory::from_archive(&mut wad).unwrap();
        let level_name = *wad.get_level_name(level_index);
        let level = Level::new(&mut wad, &textures, &level_name);

        check_gl!(gl::ClearColor(0.64, 0.72, 0.8, 0.0));
        check_gl!(gl::Enable(gl::DEPTH_TEST));
        check_gl!(gl::DepthFunc(gl::LESS));
        let mut player = Player::new(Default::default());
        {
            let start = level.get_start_pos();
            player.set_position(&Vec3::new(start.x, 0.3, start.y));
        }

        Scene { player: player, level: level }
    }

    fn update(&mut self, delta_time: f32, ctrl: &GameController) {
        self.player.update(delta_time, ctrl);
        self.level.render(
            delta_time,
            &self.player.get_camera()
            .multiply_transform(&Mat4::new_identity()));
    }
}

fn main() {
    let args: Vec<String> = os::args();
    let opts = [
        optopt("i", "iwad", "set initial wad file to use (eg doom1.wad)", "FILE"),
        optopt("l", "level", "the number of the level to render (0 indexed)", "N"),
        optflag("h", "help", "print this help message and exit"),
    ];

    let matches = match getopts(args.tail(), opts) {
        Ok(m) => m,
        Err(f) => fail!(f.to_string()),
    };

    if matches.opt_present("h") {
        println!("{}", usage("A rust doom renderer.", &opts));
        return;
    }

    let wad_filename = matches.opt_str("i").unwrap_or("doom1.wad".to_string());
    let level = matches.opt_str("l")
                       .and_then(|l| from_str::<uint>(l.as_slice()))
                       .unwrap_or(0);


    {
        let window = create_opengl_window("Rusty Doom v0.0.3", 2560, 2560*9/16);
        let _gl_context = init_opengl(&window);


        let mut scene = Scene::new(wad_filename.as_slice(), level);
        let mut control = ctrl::GameController::new();
        let quit_gesture = ctrl::AnyGesture(
            vec![ctrl::QuitTrigger,
                 ctrl::KeyTrigger(scancode::EscapeScanCode)]);

        let mut cum_time = 0.0;
        let mut cum_updates_time = 0.0;
        let mut num_frames = 0u32;
        let mut t0 = 0.0;
        loop {
            let t1 = time::precise_time_s();
            let mut delta = t1 - t0;
            if delta < 1e-10 { delta = 1.0 / 60.0; }
            let delta = delta;
            t0 = t1;
            check_gl!(gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT));

            let updates_t0 = time::precise_time_s();

            control.update();
            if control.poll_gesture(&quit_gesture) {
                break;
            }
            scene.update(delta as f32, &control);

            let updates_t1 = time::precise_time_s();
            cum_updates_time += updates_t1 - updates_t0;

            cum_time += delta;
            num_frames += 1;
            if cum_time > 2.0 {
                let fps = num_frames as f64 / cum_time;
                let cpums = 1000.0 * cum_updates_time / num_frames as f64;
                info!("Frame time: {:.2}ms ({:.2}ms cpu, FPS: {:.2})",
                      1000.0 / fps, cpums, fps);
                cum_time = 0.0;
                cum_updates_time = 0.0;
                num_frames = 0;
            }

            window.gl_swap_window();
        }
    }
    println!("main: all tasks terminated, shutting down.");
    sdl2::quit();
}


//#[start]
//fn start(argc: int, argv: *const *const u8) -> int {
//    native::start(argc, argv, main)
//}

