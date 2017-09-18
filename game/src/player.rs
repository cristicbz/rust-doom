use super::level::Level;

use engine::{Analog2d, Gesture, Input, Scancode, Camera};
use math::{Sphere, Vec3f, Vector};
use num::Zero;
use std::f32::consts::FRAC_PI_2;

pub struct PlayerBindings {
    pub movement: Analog2d,
    pub look: Analog2d,
    pub jump: Gesture,
    pub fly: Gesture,
    pub clip: Gesture,
}

impl Default for PlayerBindings {
    fn default() -> PlayerBindings {
        PlayerBindings {
            movement: Analog2d::Gestures {
                x_positive: Gesture::KeyHold(Scancode::D),
                x_negative: Gesture::KeyHold(Scancode::A),
                y_positive: Gesture::KeyHold(Scancode::W),
                y_negative: Gesture::KeyHold(Scancode::S),
                step: 1.0,
            },
            look: Analog2d::Sum {
                analogs: vec![
                    Analog2d::Gestures {
                        x_positive: Gesture::KeyHold(Scancode::Right),
                        x_negative: Gesture::KeyHold(Scancode::Left),
                        y_positive: Gesture::KeyHold(Scancode::Down),
                        y_negative: Gesture::KeyHold(Scancode::Up),
                        step: 0.05,
                    },
                    Analog2d::Mouse { sensitivity: 0.004 },
                ],
            },
            jump: Gesture::KeyHold(Scancode::Space),
            fly: Gesture::KeyTrigger(Scancode::F),
            clip: Gesture::KeyTrigger(Scancode::C),
        }
    }
}


pub struct Player {
    bindings: PlayerBindings,
    velocity: Vec3f,
    move_force: f32,
    spring_const_p: f32,
    spring_const_d: f32,
    last_height_diff: f32,
    radius: f32,
    height: f32,
    relative_camera_height: f32,
    fly: bool,
    clip: bool,
    air_drag: f32,
    ground_drag: f32,
    friction: f32,
}


impl Player {
    pub fn new(bindings: PlayerBindings, camera: &mut Camera, level: &Level) -> Player {
        let mut player = Player {
            bindings: bindings,
            velocity: Vec3f::zero(),
            move_force: 60.0,
            spring_const_p: 200.0,
            spring_const_d: 22.4,
            radius: 0.19,
            height: 0.21,
            relative_camera_height: 0.12,
            air_drag: 0.02,
            ground_drag: 0.7,
            friction: 30.0,
            fly: false,
            clip: true,
            last_height_diff: 0.0,
        };

        player.set_position(camera, level.start_pos());
        camera.set_yaw(level.start_yaw());

        player
    }

    pub fn set_position(&mut self, camera: &mut Camera, new_pos: &Vec3f) {
        camera.set_position(*new_pos + Vec3f::new(0.0, self.relative_camera_height, 0.0));
    }

    fn head(&self, camera: &Camera) -> Sphere {
        return Sphere {
            center: *camera.position() - Vec3f::new(0.0, self.relative_camera_height, 0.0),
            radius: self.radius,
        };
    }

    pub fn update(&mut self, delta_time: f32, input: &Input, level: &Level, camera: &mut Camera) {
        if input.poll_gesture(&self.bindings.fly) {
            self.fly = !self.fly;
        }

        if input.poll_gesture(&self.bindings.clip) {
            self.clip = !self.clip;
        }

        let mut head = self.head(camera);
        let force = self.force(camera, &head, delta_time, input, level);
        if self.clip {
            self.clip(delta_time, &mut head, level);
        } else {
            self.noclip(delta_time, &mut head, level);
        }

        self.set_position(camera, &head.center);
        self.velocity = self.velocity + force * delta_time;
    }

    fn clip(&mut self, delta_time: f32, head: &mut Sphere, level: &Level) {
        let mut time_left = delta_time;
        for _ in 0..100 {
            let displacement = self.velocity * time_left;
            if let Some(contact) = level.volume().sweep_sphere(head, &displacement) {
                let adjusted_time = contact.time - 0.001 / displacement.norm();
                if adjusted_time < 1.0 {
                    let time = clamp(contact.time, (0.0, 1.0));
                    let displacement = displacement * adjusted_time;
                    head.center = head.center + displacement;
                    self.velocity = self.velocity -
                        contact.normal * contact.normal.dot(&self.velocity);
                    time_left *= 1.0 - time;
                    continue;
                }
            }
            head.center = head.center + displacement;
            break;
        }
    }

    fn noclip(&mut self, delta_time: f32, head: &mut Sphere, level: &Level) {
        let old_height = head.center[1];
        head.center = head.center + self.velocity * delta_time;

        if !self.fly {
            let height = 2000.0;
            let probe = Sphere {
                center: head.center + Vec3f::new(0.0, height / 2.0, 0.0),
                ..*head
            };
            let height = match level.volume().sweep_sphere(
                &probe,
                &Vec3f::new(0.0, -height, 0.0),
            ) {
                Some(contact) => head.center[1] + height * (0.5 - contact.time),
                None => old_height,
            };

            if head.center[1] <= height {
                head.center[1] = height;
                if self.velocity[1] < 0.0 {
                    self.velocity[1] = 0.0;
                }
            }
        }
    }

    fn move_force(
        &mut self,
        camera: &mut Camera,
        delta_time: f32,
        grounded: bool,
        input: &Input,
    ) -> Vec3f {
        let movement = input.poll_analog2d(&self.bindings.movement);
        let look = input.poll_analog2d(&self.bindings.look);
        let jump = input.poll_gesture(&self.bindings.jump);
        let yaw = camera.yaw() + look[0];
        let pitch = clamp(camera.pitch() + look[1], (-FRAC_PI_2, FRAC_PI_2));
        camera.set_yaw(yaw);
        camera.set_pitch(pitch);

        if self.fly {
            let up = if jump { 0.5 } else { 0.0 };
            Vec3f::new(
                yaw.cos() * movement[0] + yaw.sin() * movement[1] * pitch.cos(),
                -pitch.sin() * movement[1] + up,
                -yaw.cos() * movement[1] * pitch.cos() + yaw.sin() * movement[0],
            ).normalized() * self.move_force
        } else {
            let movement = Vec3f::new(
                yaw.cos() * movement[0] + yaw.sin() * movement[1],
                0.0,
                -yaw.cos() * movement[1] + yaw.sin() * movement[0],
            ).normalized() * self.move_force;
            if grounded {
                if jump && self.velocity[1] < 0.1 {
                    Vec3f::new(movement[0], 5.0 / delta_time, movement[2])
                } else {
                    movement
                }
            } else {
                movement * 0.1
            }
        }
    }

    fn force(
        &mut self,
        camera: &mut Camera,
        head: &Sphere,
        delta_time: f32,
        input: &Input,
        level: &Level,
    ) -> Vec3f {
        let feet = Sphere {
            radius: 0.2,
            ..*head
        };
        let feet_probe = Vec3f::new(0.0, -self.height, 0.0);
        let (height, normal) =
            if let Some(contact) = level.volume().sweep_sphere(&feet, &feet_probe) {
                if contact.time < 1.0 {
                    (self.height * contact.time, Some(contact.normal))
                } else {
                    (self.height, None)
                }
            } else {
                (self.height, None)
            };
        let mut force: Vec3f = self.move_force(camera, delta_time, normal.is_some(), input);
        let speed = self.velocity.norm();
        if speed > 0.0 {
            let mut slowdown = if self.fly {
                -self.velocity * (self.friction / speed + self.ground_drag * speed)
            } else if let Some(normal) = normal {
                let tangential = self.velocity - normal * self.velocity.dot(&normal);
                let speed = tangential.norm();
                if speed > 0.0 {
                    -tangential * (self.friction / speed + self.ground_drag * speed)
                } else {
                    Vec3f::zero()
                }
            } else {
                Vec3f::zero()
            };
            slowdown = slowdown - self.velocity * self.air_drag * speed;

            let slowdown_norm = slowdown.norm();
            if slowdown_norm > 0.0 {
                let max_slowdown = -self.velocity.dot(&slowdown) / slowdown_norm / delta_time;
                if slowdown_norm >= max_slowdown {
                    slowdown = slowdown / slowdown_norm * max_slowdown;
                }
                force = force + slowdown;
            }
        }
        let height_diff = self.height - height;
        let derivative = (height_diff - self.last_height_diff) / delta_time;
        self.last_height_diff = height_diff;
        force[1] += height_diff * self.spring_const_p + derivative * self.spring_const_d;

        if !self.fly {
            force[1] -= 17.0
        }
        force
    }
}

fn clamp<T: PartialOrd>(value: T, (limit_min, limit_max): (T, T)) -> T {
    if value < limit_min {
        limit_min
    } else if value > limit_max {
        limit_max
    } else {
        value
    }
}
