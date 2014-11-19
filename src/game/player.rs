use camera::Camera;
use ctrl::GameController;
use ctrl::{Analog2d, Gesture};
use math::{Vec3, Vec3f, Vec2f, Numvec};
use sdl2::scancode::ScanCode;
use std::default::Default;
use std::num::FloatMath;


pub struct PlayerBindings {
    pub movement: Analog2d,
    pub look: Analog2d,
}


impl PlayerBindings {
    pub fn look_vector(&self, controller: &GameController) -> Vec2f {
        controller.poll_analog2d(&self.look)
    }
    pub fn movement_vector(&self, controller: &GameController) -> Vec2f {
        controller.poll_analog2d(&self.movement)
    }
}


impl Default for PlayerBindings {
    fn default() -> PlayerBindings {
        PlayerBindings {
            movement: Analog2d::Gestures(
                Gesture::KeyHold(ScanCode::D),
                Gesture::KeyHold(ScanCode::A),
                Gesture::KeyHold(ScanCode::W),
                Gesture::KeyHold(ScanCode::S),
                1.0),
            look: Analog2d::Mouse(0.002)
        }
    }
}


pub struct Player {
    bindings: PlayerBindings,
    camera: Camera,
    movement_speed: f32,
}


impl Player {
    pub fn new(fov: f32, aspect_ratio: f32,
               bindings: PlayerBindings) -> Player {
        let mut camera = Camera::new(fov, aspect_ratio, 0.1, 100.0);
        camera.set_yaw(3.1415926538);

        Player {
            bindings: bindings,
            camera: camera,
            movement_speed: 8.0,
        }
    }

    pub fn set_position(&mut self, new_pos: &Vec3f) -> &mut Player {
        self.camera.set_position(*new_pos);
        self
    }

    pub fn update(&mut self, delta_time: f32, controller: &GameController) {
        let movement = self.bindings.movement_vector(controller);
        let look = self.bindings.look_vector(controller);

        if movement.norm() == 0.0 && look.norm() == 0.0 { return; }

        let yaw = self.camera.get_yaw() + look.x;
        let pitch = clamp(self.camera.get_pitch() - look.y,
                          (-3.14 / 2.0, 3.14 / 2.0));

        let displacement = self.movement_speed * delta_time;
        let movement : Vec3f = Vec3::new(
            yaw.cos() * movement.x * displacement +
             yaw.sin() * movement.y * displacement * pitch.cos(),
            -pitch.sin() * movement.y * displacement,
            -yaw.cos() * movement.y * displacement * pitch.cos()
             + yaw.sin() * movement.x * displacement);
        self.camera.set_yaw(yaw);
        self.camera.set_pitch(pitch);
        self.camera.move_by(movement);

        if movement.norm() > 0.0 {
            //info!("Pos: {}", self.camera.get_position())
        }
    }

    pub fn get_camera(&self) -> &Camera {
        &self.camera
    }

    pub fn get_camera_mut(&mut self) -> &mut Camera {
        &mut self.camera
    }
}

fn clamp<T : PartialOrd>(value: T, (limit_min, limit_max): (T, T)) -> T {
    if value < limit_min { limit_min }
    else if value > limit_max { limit_max }
    else { value }
}

