use wad::{LightEffectKind, LightInfo};

pub struct Lights {
    lights: Vec<LightInfo>,
}

impl Lights {
    pub fn new() -> Self {
        Self {
            lights: Vec::with_capacity(256),
        }
    }

    pub fn push(&mut self, light_info: &LightInfo) -> u8 {
        self.lights
            .iter()
            .position(|x| x == light_info)
            .unwrap_or_else(|| {
                // TODO(cristicbz): Remove this restriction.
                assert!(self.lights.len() < 255);
                self.lights.push(light_info.clone());
                (self.lights.len() - 1)
            }) as u8
    }

    pub fn fill_buffer_at(&mut self, time: f32, buffer: &mut [u8]) {
        for (value, info) in buffer.iter_mut().zip(self.lights.iter()) {
            *value = (clamp(light_level_at(info, time)) * 255.0) as u8;
        }
    }
}

fn light_level_at(info: &LightInfo, time: f32) -> f32 {
    let effect = if let Some(ref effect) = info.effect {
        effect
    } else {
        return info.level;
    };
    match effect.kind {
        LightEffectKind::Glow => {
            let scale = info.level - effect.alt_level;
            let phase = time * effect.speed / scale;
            (0.5 - fract(phase)).abs() * 2.0 * scale + effect.alt_level
        }
        LightEffectKind::Random => {
            if noise(effect.sync, (time * effect.speed).floor()) < effect.duration {
                effect.alt_level
            } else {
                info.level
            }
        }
        LightEffectKind::Alternate => {
            if fract(time * effect.speed + effect.sync * 3.5435) < effect.duration {
                effect.alt_level
            } else {
                info.level
            }
        }
    }
}

fn noise(sync: f32, time: f32) -> f32 {
    fract(1.0 + ((sync + time / 1000.0) * 12.9898 + sync * 78.233).sin() * 43758.547)
}

fn fract(x: f32) -> f32 {
    x - x.floor()
}

fn clamp(x: f32) -> f32 {
    if x < 0.0 {
        0.0
    } else if x > 1.0 {
        1.0
    } else {
        x
    }
}
