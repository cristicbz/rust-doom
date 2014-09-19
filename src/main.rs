#![feature(macro_rules)]
#![feature(phase)]
#[phase(plugin, link)] extern crate log;

extern crate sdl2;
extern crate serialize;
extern crate gl;
extern crate libc;
extern crate native;
extern crate zmq;

use ctrl::GameController;
use gl::types::GLuint;
use level::Level;
use libc::c_void;
use mat4::Mat4;
use player::Player;
use sdl2::scancode;
use shader::{Shader, Uniform};
use std::default::Default;
use wad::WadFile;
use numvec::Vec3;


#[macro_escape]
pub mod check_gl;

pub mod async_term;
pub mod camera;
pub mod ctrl;
pub mod mat4;
pub mod numvec;
pub mod player;
pub mod shader;
pub mod shlex;
pub mod wad;
pub mod level;
pub mod vbo;

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
    fn new() -> Scene {
        let mut wad = WadFile::open(&Path::new("doom1.wad")).unwrap();
        let level_name: [u8, ..8] =
            [b'E', b'1', b'M', b'1', b'\0', b'\0', b'\0', b'\0'];
        let level = Level::new(&mut wad, &level_name);

        check_gl!(gl::ClearColor(0.0, 0.1, 0.4, 0.0));
        check_gl!(gl::Enable(gl::DEPTH_TEST));
        check_gl!(gl::DepthFunc(gl::LESS));
        let mut player = Player::new(Default::default());
        {
            let start = level.get_start_pos();
            player.set_position(&Vec3::new(start.x, 1.0, start.y));
        }

        Scene { player: player, level: level }
    }

    fn update(&mut self, ctrl : &GameController) {
        check_gl!(gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT));

        // Update player.
        self.player.update(0.1, ctrl);
        self.level.render(
            &self.player.get_camera()
            .multiply_transform(&Mat4::new_identity()));

    }
}

fn main() {
    {
        let window = create_opengl_window("thingy", 1920, 1080);
        let _gl_context = init_opengl(&window);
        let mut scene = Scene::new();
        let mut control = ctrl::GameController::new();
        let quit_gesture = ctrl::AnyGesture(
            vec![ctrl::QuitTrigger,
                 ctrl::KeyTrigger(scancode::EscapeScanCode)]);

        loop {
            control.update();
            scene.update(&control);
            window.gl_swap_window();
            if control.poll_gesture(&quit_gesture) {
                break;
            }
        }
    }
    println!("main: all tasks terminated, shutting down.");
    sdl2::quit();
}


//#[start]
//fn start(argc: int, argv: *const *const u8) -> int {
//    native::start(argc, argv, main)
//}

