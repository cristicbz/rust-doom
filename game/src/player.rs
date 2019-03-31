use super::level::{Level, PlayerAction};
use engine::{
    Analog2d, DependenciesFrom, Entities, EntityId, Gesture, InfallibleSystem, Input, MouseButton,
    Projection, Projections, RenderPipeline, Scancode, Tick, Transforms, Window,
};
use log::error;
use math::prelude::*;
use math::{vec3, Deg, Euler, Pnt3f, Quat, Rad, Sphere, Trans3, Vec3f};
use std::f32::consts::FRAC_PI_2;

pub struct Bindings {
    pub movement: Analog2d,
    pub look: Analog2d,
    pub jump: Gesture,
    pub fly: Gesture,
    pub clip: Gesture,
    pub push: Gesture,
    pub shoot: Gesture,
}

impl Default for Bindings {
    fn default() -> Bindings {
        Bindings {
            movement: Analog2d::Gestures {
                x_positive: Gesture::KeyHold(Scancode::D),
                x_negative: Gesture::KeyHold(Scancode::A),
                y_positive: Gesture::KeyHold(Scancode::S),
                y_negative: Gesture::KeyHold(Scancode::W),
                step: 1.0,
            },
            look: Analog2d::Sum {
                analogs: vec![
                    Analog2d::Gestures {
                        x_positive: Gesture::KeyHold(Scancode::Right),
                        x_negative: Gesture::KeyHold(Scancode::Left),
                        y_positive: Gesture::KeyHold(Scancode::Down),
                        y_negative: Gesture::KeyHold(Scancode::Up),
                        step: 0.015,
                    },
                    Analog2d::Mouse {
                        sensitivity: 0.0015,
                    },
                ],
            },
            jump: Gesture::KeyHold(Scancode::Space),
            push: Gesture::KeyTrigger(Scancode::E),
            shoot: Gesture::ButtonTrigger(MouseButton::Left),
            fly: Gesture::KeyTrigger(Scancode::F),
            clip: Gesture::KeyTrigger(Scancode::C),
        }
    }
}

pub struct Config {
    move_force: f32,
    spring_const_p: f32,
    spring_const_d: f32,
    radius: f32,
    height: f32,
    air_drag: f32,
    ground_drag: f32,
    friction: f32,

    fov: Deg<f32>,
    near: f32,
    far: f32,
    aspect_ratio_correction: f32,

    camera_height: f32,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            move_force: 60.0,
            spring_const_p: 200.0,
            spring_const_d: 22.4,
            radius: 0.19,
            height: 0.21,
            air_drag: 0.02,
            ground_drag: 0.7,
            friction: 30.0,

            fov: Deg(65.0),
            near: 0.01,
            far: 100.0,
            aspect_ratio_correction: 1.2,

            camera_height: 0.12,
        }
    }
}

#[derive(DependenciesFrom)]
pub struct Dependencies<'context> {
    bindings: &'context Bindings,
    config: &'context Config,

    tick: &'context Tick,
    window: &'context Window,
    input: &'context Input,
    entities: &'context mut Entities,
    transforms: &'context mut Transforms,
    projections: &'context mut Projections,
    render: &'context mut RenderPipeline,

    level: &'context mut Level,
}

pub struct Player {
    id: EntityId,
    velocity: Vec3f,
    fly: bool,
    clip: bool,
    last_height_diff: f32,
}

impl Player {
    fn reset(&mut self, transforms: &mut Transforms, level: &Level) {
        let transform = transforms
            .get_local_mut(self.id)
            .expect("player has no transform component: reset");

        transform.rot = Quat::from(Euler {
            x: Rad(1e-8),
            y: level.start_yaw(),
            z: Rad(0.0),
        });
        transform.disp = level.start_pos().to_vec();

        self.velocity = Vec3f::zero();
        self.last_height_diff = 0.0;
    }

    fn head(&self, config: &Config, transform: &Trans3) -> Sphere {
        Sphere {
            center: Pnt3f::from_vec(transform.disp),
            radius: config.radius,
        }
    }

    fn clip(&mut self, delta_time: f32, head: &mut Sphere, level: &Level) {
        let mut time_left = delta_time;
        let mut armed = true;
        for _ in 0..100 {
            let displacement = self.velocity * time_left;
            if let Some(contact) = level.volume().sweep_sphere(*head, displacement) {
                let adjusted_time = contact.time - 0.001 / displacement.magnitude();
                if adjusted_time < 1.0 {
                    let time = clamp(contact.time, (0.0, 1.0));
                    let displacement = displacement * adjusted_time;
                    head.center += displacement;
                    self.velocity -= contact.normal * contact.normal.dot(self.velocity);
                    time_left *= 1.0 - time;
                    continue;
                }
            }
            head.center += displacement;
            armed = false;
            break;
        }

        if armed {
            error!("Failed to compute collisions.");
        }
    }

    fn noclip(&mut self, delta_time: f32, head: &mut Sphere, level: &Level) {
        let old_height = head.center[1];
        head.center += self.velocity * delta_time;

        if !self.fly {
            let height = 2000.0;
            let probe = Sphere {
                center: head.center + Vec3f::new(0.0, height / 2.0, 0.0),
                ..*head
            };
            let height = match level.volume().sweep_sphere(probe, vec3(0.0, -height, 0.0)) {
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
        delta_time: f32,
        grounded: bool,
        input: &Input,
        transform: &mut Trans3,
        config: &Config,
        bindings: &Bindings,
    ) -> Vec3f {
        let movement = input.poll_analog2d(&bindings.movement);
        let look = input.poll_analog2d(&bindings.look);
        let jump = input.poll_gesture(&bindings.jump);

        // Compute the maximum pitch rotation we're allowed (since we don't want to look
        // upside-down!).
        let current_pitch = 2.0 * f32::atan(transform.rot.v.x / transform.rot.s);
        let clamped_pitch_by = clamp(
            -look[1],
            (
                1e-2 - FRAC_PI_2 - current_pitch,
                FRAC_PI_2 - 1e-2 - current_pitch,
            ),
        );
        transform.rot = Quat::from_angle_y(Rad(-look.x))
            * transform.rot
            * Quat::from_angle_x(Rad(clamped_pitch_by));

        if self.fly {
            let up = if jump { 0.5 } else { 0.0 };
            transform.rot.rotate_vector(
                vec3(movement[0], up, movement[1]).normalize_or_zero() * config.move_force,
            )
        } else {
            let mut movement = transform
                .rot
                .rotate_vector(vec3(movement[0], 0.0, movement[1]));
            movement[1] = 0.0;
            movement.normalize_or_zero_self();
            movement *= config.move_force;
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
        head: &Sphere,
        delta_time: f32,
        input: &Input,
        level: &Level,
        transform: &mut Trans3,
        config: &Config,
        bindings: &Bindings,
    ) -> Vec3f {
        let feet = Sphere {
            radius: 0.2,
            ..*head
        };
        let feet_probe = Vec3f::new(0.0, -config.height, 0.0);
        let (height, normal) = if let Some(contact) = level.volume().sweep_sphere(feet, feet_probe)
        {
            if contact.time < 1.0 {
                (config.height * contact.time, Some(contact.normal))
            } else {
                (config.height, None)
            }
        } else {
            (config.height, None)
        };
        let mut force: Vec3f = self.move_force(
            delta_time,
            normal.is_some(),
            input,
            transform,
            config,
            bindings,
        );
        let speed = self.velocity.magnitude();
        if speed > 0.0 {
            let mut slowdown = if self.fly {
                -self.velocity * (config.friction / speed + config.ground_drag * speed)
            } else if let Some(normal) = normal {
                let tangential = self.velocity - normal * self.velocity.dot(normal);
                let speed = tangential.magnitude();
                if speed > 0.0 {
                    -tangential * (config.friction / speed + config.ground_drag * speed)
                } else {
                    Vec3f::zero()
                }
            } else {
                Vec3f::zero()
            };
            slowdown -= self.velocity * config.air_drag * speed;

            let slowdown_norm = slowdown.magnitude();
            if slowdown_norm > 0.0 {
                let max_slowdown = -self.velocity.dot(slowdown) / slowdown_norm / delta_time;
                if slowdown_norm >= max_slowdown {
                    slowdown = slowdown / slowdown_norm * max_slowdown;
                }
                force += slowdown;
            }
        }
        let height_diff = config.height - height;
        let derivative = (height_diff - self.last_height_diff) / delta_time;
        self.last_height_diff = height_diff;
        force[1] += height_diff * config.spring_const_p + derivative * config.spring_const_d;

        if !self.fly {
            force[1] -= 17.0
        }
        force
    }
}

impl<'context> InfallibleSystem<'context> for Player {
    type Dependencies = Dependencies<'context>;

    fn debug_name() -> &'static str {
        "player"
    }

    fn create(deps: Dependencies) -> Player {
        let player_entity = deps.entities.add_root("player");
        deps.transforms.attach_identity(player_entity);

        let camera_entity = deps
            .entities
            .add(player_entity, "camera")
            .expect("failed to add camera to fresh entity");
        deps.transforms.attach(
            camera_entity,
            Trans3 {
                disp: Vec3f::new(0.0, deps.config.camera_height, 0.0),
                ..Trans3::one()
            },
        );
        deps.projections.attach(
            camera_entity,
            Projection {
                fov: deps.config.fov.into(),
                aspect_ratio: deps.window.aspect_ratio() * deps.config.aspect_ratio_correction,
                near: deps.config.near,
                far: deps.config.far,
            },
        );
        deps.render.set_camera(camera_entity);

        let mut player = Player {
            id: player_entity,
            velocity: Vec3f::zero(),
            fly: false,
            clip: true,
            last_height_diff: 0.0,
        };

        player.reset(deps.transforms, deps.level);
        player
    }

    fn update(&mut self, deps: Dependencies) {
        if deps.level.level_changed() {
            self.reset(deps.transforms, deps.level);
        }

        let delta_time = deps.tick.timestep();
        let transform = deps
            .transforms
            .get_local_mut(self.id)
            .expect("player has no transform component: update");

        if deps.input.poll_gesture(&deps.bindings.fly) {
            self.fly = !self.fly;
        }

        if deps.input.poll_gesture(&deps.bindings.clip) {
            self.clip = !self.clip;
        }

        let mut head = self.head(deps.config, transform);
        let force = self.force(
            &head,
            delta_time,
            deps.input,
            deps.level,
            transform,
            deps.config,
            deps.bindings,
        );
        if self.clip {
            self.clip(delta_time, &mut head, deps.level);
        } else {
            self.noclip(delta_time, &mut head, deps.level);
        }

        transform.disp = head.center.to_vec();
        self.velocity += force * delta_time;

        deps.level.poll_triggers(
            transform,
            self.velocity * delta_time,
            if deps.input.poll_gesture(&deps.bindings.push) {
                Some(PlayerAction::Push)
            } else if deps.input.poll_gesture(&deps.bindings.shoot) {
                Some(PlayerAction::Shoot)
            } else {
                None
            },
        );
    }

    fn teardown(&mut self, deps: Dependencies) {
        deps.entities.remove(self.id);
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
