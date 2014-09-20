use camera::Camera;
use ctrl;
use ctrl::GameController;
use numvec::{Vec3, Vec3f, Vec2f, Numvec};
use sdl2::scancode;
use std::default::Default;


pub struct PlayerBindings {
    pub move: ctrl::Analog2d,
    pub look: ctrl::Analog2d,
}


impl PlayerBindings {
    pub fn look_vector(&self, controller: &GameController) -> Vec2f {
        controller.poll_analog2d(&self.look)
    }
    pub fn move_vector(&self, controller: &GameController) -> Vec2f {
        controller.poll_analog2d(&self.move)
    }
}


impl Default for PlayerBindings {
    fn default() -> PlayerBindings {
        PlayerBindings {
            move: ctrl::GesturesAnalog2d(
                ctrl::KeyHold(scancode::DScanCode),
                ctrl::KeyHold(scancode::AScanCode),
                ctrl::KeyHold(scancode::WScanCode),
                ctrl::KeyHold(scancode::SScanCode),
                1.0),
            look: ctrl::MouseMotion(0.002)
        }
    }
}


pub struct Player {
    bindings: PlayerBindings,
    camera: Camera,
    move_speed: f32,
}


impl Player {
    pub fn new(bindings: PlayerBindings) -> Player {
        let mut camera = Camera::new(65.0, 16.0 / 9.0, 0.1, 100.0);
        camera.set_yaw(3.1415926538);

        Player {
            bindings: bindings,
            camera: camera,
            move_speed: 4.0,
        }
    }

    pub fn set_position(&mut self, new_pos: &Vec3f) -> &mut Player {
        self.camera.set_position(*new_pos);
        self
    }

    pub fn update(&mut self, delta_time: f32, controller: &GameController) {
        let move = self.bindings.move_vector(controller);
        let look = self.bindings.look_vector(controller);

        if move.norm() == 0.0 && look.norm() == 0.0 { return; }

        let yaw = self.camera.get_yaw() + look.x;
        let pitch = clamp(self.camera.get_pitch() - look.y,
                          (-3.14 / 2.0, 3.14 / 2.0));

        let displacement = self.move_speed * delta_time;
        let move : Vec3f = Vec3::new(
            yaw.cos() * move.x * displacement +
             yaw.sin() * move.y * displacement * pitch.cos(),
            -pitch.sin() * move.y * displacement,
            -yaw.cos() * move.y * displacement * pitch.cos()
             + yaw.sin() * move.x * displacement);
        self.camera.set_yaw(yaw);
        self.camera.set_pitch(pitch);
        self.camera.move_by(move);

        if move.norm() > 0.0 {
            //info!("Pos: {}", self.camera.get_position())
        }
    }

    pub fn get_camera<'a>(&'a self) -> &'a Camera {
        &self.camera
    }

    pub fn get_camera_mut<'a>(&'a mut self) -> &'a mut Camera {
        &mut self.camera
    }
}

fn clamp<T : PartialOrd>(value: T, (limit_min, limit_max): (T, T)) -> T {
    if value < limit_min { limit_min }
    else if value > limit_max { limit_max }
    else { value }
}

