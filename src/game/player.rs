use camera::Camera;
use ctrl::{Analog2d, Gesture};
use ctrl::GameController;
use level::Level;
use math::{Vec3, Vec3f, Vec2f, Numvec};
use sdl2::scancode::ScanCode;
use std::default::Default;
use num::Float;


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
                Gesture::AnyOf(vec![Gesture::KeyHold(ScanCode::D),
                                    Gesture::KeyHold(ScanCode::Right)]),
                Gesture::AnyOf(vec![Gesture::KeyHold(ScanCode::A),
                                    Gesture::KeyHold(ScanCode::Left)]),
                Gesture::AnyOf(vec![Gesture::KeyHold(ScanCode::W),
                                    Gesture::KeyHold(ScanCode::Up)]),
                Gesture::AnyOf(vec![Gesture::KeyHold(ScanCode::S),
                                    Gesture::KeyHold(ScanCode::Down)]),
                1.0),
            look: Analog2d::Mouse(0.002)
        }
    }
}


pub struct Player {
    bindings: PlayerBindings,
    camera: Camera,
    movement_speed: f32,
    target_height: f32,
    vertical_speed: f32,
}


impl Player {
    pub fn new(fov: f32, aspect_ratio: f32,
               bindings: PlayerBindings) -> Player {
        let mut camera = Camera::new(fov, aspect_ratio, 0.01, 100.0);
        camera.set_yaw(3.1415926538);

        Player {
            bindings: bindings,
            camera: camera,
            movement_speed: 5.0,
            target_height: 0.0,
            vertical_speed: 0.0,
        }
    }

    pub fn set_position(&mut self, new_pos: &Vec3f) -> &mut Player {
        self.camera.set_position(*new_pos);
        self
    }

    pub fn update(&mut self, delta_time: f32, controller: &GameController, level: &Level) {
        let movement = self.bindings.movement_vector(controller);
        let look = self.bindings.look_vector(controller);

        if movement.norm() != 0.0 || look.norm() != 0.0 {
            let yaw = self.camera.yaw() + look.x;
            let pitch = clamp(self.camera.pitch() - look.y, (-3.14 / 2.0, 3.14 / 2.0));

            let displacement = self.movement_speed * delta_time;
            //let movement: Vec3f = Vec3::new(
            //    yaw.cos() * movement.x * displacement +
            //     yaw.sin() * movement.y * displacement * pitch.cos(),
            //    -pitch.sin() * movement.y * displacement,
            //    -yaw.cos() * movement.y * displacement * pitch.cos()
            //     + yaw.sin() * movement.x * displacement);
            let movement: Vec3f = Vec3::new(
                yaw.cos() * movement.x * displacement + yaw.sin() * movement.y * displacement,
                0.0,
                -yaw.cos() * movement.y * displacement + yaw.sin() * movement.x * displacement);
            self.camera.set_yaw(yaw);
            self.camera.set_pitch(pitch);
            self.camera.move_by(movement);
        }

        let mut pos = *self.camera.position();
        level.floor_at(&Vec2f::new(pos.x, pos.z)).map(|floor| {
            self.target_height = floor + 50.0 / 100.0;
        });

        let old_y = pos.y;
        pos.y += self.vertical_speed * delta_time;
        if old_y < self.target_height && pos.y > self.target_height
                || old_y > self.target_height && pos.y < self.target_height {
            self.vertical_speed = 0.0;
            pos.y = self.target_height;
        } else if (pos.y - self.target_height).abs() > 1e-3 {
            if pos.y < self.target_height {
                if self.target_height - pos.y > 1.0 {
                    pos.y = self.target_height;
                } else {
                    self.vertical_speed += 1.0 * delta_time;
                    pos.y = (pos.y + self.target_height + 0.1)/2.0;
                }
            } else {
                self.vertical_speed -= 20.0 * delta_time;
            }
        }
        self.camera.set_position(pos);
    }

    pub fn get_camera(&self) -> &Camera {
        &self.camera
    }

    pub fn get_camera_mut(&mut self) -> &mut Camera {
        &mut self.camera
    }
}

fn clamp<T: PartialOrd>(value: T, (limit_min, limit_max): (T, T)) -> T {
    if value < limit_min { limit_min }
    else if value > limit_max { limit_max }
    else { value }
}

