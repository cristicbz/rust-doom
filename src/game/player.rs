use camera::Camera;
use ctrl::{Analog2d, Gesture};
use ctrl::GameController;
use level::Level;
use math::{Vec3f, Vec2f, Numvec};
use sdl2::keyboard::Scancode;
use std::default::Default;
use num::Float;


pub struct PlayerBindings {
    pub movement: Analog2d,
    pub look: Analog2d,
    pub jump: Gesture,
}


impl PlayerBindings {
    pub fn look_vector(&self, controller: &GameController) -> Vec2f {
        controller.poll_analog2d(&self.look)
    }

    pub fn movement_vector(&self, controller: &GameController) -> Vec2f {
        controller.poll_analog2d(&self.movement)
    }

    pub fn jump(&self, controller: &GameController) -> bool {
        controller.poll_gesture(&self.jump)
    }
}


impl Default for PlayerBindings {
    fn default() -> PlayerBindings {
        PlayerBindings {
            movement: Analog2d::Gestures(
                Gesture::AnyOf(vec![Gesture::KeyHold(Scancode::D),
                                    Gesture::KeyHold(Scancode::Right)]),
                Gesture::AnyOf(vec![Gesture::KeyHold(Scancode::A),
                                    Gesture::KeyHold(Scancode::Left)]),
                Gesture::AnyOf(vec![Gesture::KeyHold(Scancode::W),
                                    Gesture::KeyHold(Scancode::Up)]),
                Gesture::AnyOf(vec![Gesture::KeyHold(Scancode::S),
                                    Gesture::KeyHold(Scancode::Down)]),
                1.0),
            look: Analog2d::Mouse(0.002),
            jump: Gesture::KeyTrigger(Scancode::Space),
        }
    }
}


pub struct Player {
    bindings: PlayerBindings,
    camera: Camera,
    move_accel: f32,
    floor_height: f32,
    ceil_height: f32,
    vertical_speed: f32,
    horizontal_speed: Vec2f,
}


impl Player {
    pub fn new(fov: f32, aspect_ratio: f32,
               bindings: PlayerBindings) -> Player {
        let mut camera = Camera::new(fov, aspect_ratio, 0.01, 100.0);
        camera.set_yaw(3.1415926538);

        Player {
            bindings: bindings,
            camera: camera,
            move_accel: 10.0,
            floor_height: 0.0,
            ceil_height: 100.0,
            vertical_speed: 0.0,
            horizontal_speed: Vec2f::zero(),
        }
    }

    pub fn set_position(&mut self, new_pos: &Vec3f) -> &mut Player {
        self.camera.set_position(*new_pos);
        self
    }

    pub fn update(&mut self, delta_time: f32, controller: &GameController, level: &Level) {
        let mut pos = *self.camera.position();
        let old_pos = pos;

        pos.x += self.horizontal_speed.x * delta_time;
        pos.z += self.horizontal_speed.y * delta_time;
        pos.y += self.vertical_speed * delta_time;
        level.heights_at(&Vec2f::new(pos.x, pos.z)).map(|(floor, ceil)| {
            let (floor, ceil) = (floor + 50.0 / 100.0, ceil - 1.0 / 100.0);
            let ceil = if ceil < floor { floor } else { ceil };
            self.floor_height = floor;
            self.ceil_height = ceil;
        });


        let floor_dist = pos.y - self.floor_height;
        let in_control = self.vertical_speed.abs() < 0.5 && floor_dist < 1e-1;
        let floored = floor_dist < 1e-2;

        if floored {
            self.horizontal_speed = self.horizontal_speed * 0.7;
        } else {
            self.horizontal_speed = self.horizontal_speed * 0.97;
        }

        if old_pos.y < self.floor_height && pos.y > self.floor_height
                || old_pos.y > self.floor_height && pos.y < self.floor_height
                || floor_dist.abs() <= 1e-3 {
            self.vertical_speed = 0.0;
            pos.y = self.floor_height;
        } else if pos.y > self.ceil_height {
            self.vertical_speed = 0.0;
            pos.y = self.ceil_height;
        } else {
            if pos.y < self.floor_height {
                if self.floor_height - pos.y > 1.0 {
                    pos.y = self.floor_height;
                } else {
                    self.vertical_speed += 10.0 * delta_time;
                    pos.y = pos.y * 0.7 + self.floor_height * 0.3;
                }
            } else {
                self.vertical_speed -= 17.0 * delta_time;
            }
        }

        let movement = self.bindings.movement_vector(controller);
        let look = self.bindings.look_vector(controller);
        if movement.norm() != 0.0 || look.norm() != 0.0 {
            let yaw = self.camera.yaw() + look.x;
            let pitch = clamp(self.camera.pitch() - look.y, (-3.14 / 2.0, 3.14 / 2.0));
            self.camera.set_yaw(yaw);
            self.camera.set_pitch(pitch);

            let movement = Vec2f::new(
                yaw.cos() * movement.x + yaw.sin() * movement.y,
                -yaw.cos() * movement.y + yaw.sin() * movement.x) * self.move_accel;
            let mut displacement = self.move_accel * delta_time;
            if !floored {
                displacement *= 0.05;
            }
            self.horizontal_speed = self.horizontal_speed + movement * displacement;
        }

        let jump = self.bindings.jump(controller);
        if jump && in_control {
            self.vertical_speed = 5.0;
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

