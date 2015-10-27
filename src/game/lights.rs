use wad::types::{LightLevel, SectorType, SectorId};

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum LightEffect {
    Flash,
    FastStrobe,
    SlowStrobe,
    Glow,
    Flicker,
    Constant,
}

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum FakeContrast {
    None,
    Darken,
    Brighten,
}

pub struct LightBuffer {
    lights: Vec<Light>,
    levels: Vec<f32>,
}

impl LightBuffer {
    pub fn new() -> LightBuffer {
        LightBuffer {
            lights: vec![],
            levels: vec![],
        }
    }

    pub fn push(&mut self,
            sector_light: LightLevel, alt_light: LightLevel,
            sector_type: SectorType, sector_id: SectorId,
            fake_contrast: FakeContrast) -> u8 {
        assert!(self.lights.len() < 256);
        let new_light = Light::new(sector_light, alt_light, sector_type, sector_id, fake_contrast);
        let existing_index = self.lights
            .iter()
            .enumerate()
            .find(|&(_, x)| x == &new_light)
            .map(|(i, _)| i as u8);
        if let Some(index) = existing_index {
            index
        } else {
            self.levels.push(0.0);
            self.lights.push(new_light);
            (self.lights.len() - 1) as u8
        }
    }

    pub fn fill_buffer_at(&mut self, time: f32, buffer: &mut [f32]) {
        for (value, info) in buffer.iter_mut().zip(self.lights.iter()) {
            *value = info.light_level_at(time);
        }
    }
}

const FLASH: SectorType = 1;
const FAST_STROBE_1: SectorType = 2;
const FAST_STROBE_2: SectorType = 4;
const FAST_STROBE_SYNC: SectorType = 13;
const SLOW_STROBE: SectorType = 3;
const SLOW_STROBE_SYNC: SectorType = 12;
const GLOW: SectorType = 8;
const FLICKER: SectorType = 17;

impl From<SectorType> for LightEffect {
    fn from(sector_type: SectorType) -> LightEffect {
        match sector_type {
            FLASH => LightEffect::Flash,
            FAST_STROBE_1 | FAST_STROBE_2 | FAST_STROBE_SYNC => LightEffect::FastStrobe,
            SLOW_STROBE | SLOW_STROBE_SYNC => LightEffect::SlowStrobe,
            GLOW => LightEffect::Glow,
            FLICKER => LightEffect::Flicker,
            _ => LightEffect::Constant,
        }
    }
}


#[derive(PartialEq, Debug)]
struct Light {
    level0: f32,
    level1: f32,
    sync: f32,
    effect: LightEffect,
}

impl Light {
    fn new(level0: LightLevel, level1: LightLevel, sector_type: SectorType, sector_id: SectorId,
           fake_contrast: FakeContrast) -> Light {
        let level0 = (apply_contrast(level0, fake_contrast) >> 3) as f32 / 31.0;
        let level1 = (apply_contrast(level1, fake_contrast) >> 3) as f32 / 31.0;
        let effect = sector_type.into();
        if effect == LightEffect::Constant
                || (effect == LightEffect::Glow && level0 == level1) {
            return Light {
                level0: level0,
                level1: level1,
                sync: 0.0,
                effect: LightEffect::Constant,
            };
        }

        let level1 = if level0 != level1 { level1 } else { 0.0 };
        let sync = if sector_type == 12 || sector_type == 13 || sector_type == 8 { 0.0 }
        else { ((sector_id as u64 * 1664525 + 1013904223) & 0xffff) as f32 / 15.0 };

        Light {
            level0: level0,
            level1: level1,
            sync: sync,
            effect: effect,
        }
    }

    fn noise(&self, time: f32) -> f32 {
        let r = (1.0 + ((self.sync + time / 1000.0) * 12.9898
                        + self.sync * 78.233).sin()) * 43758.5453;
        r - r.floor()
    }

    fn light_level_at(&self, time: f32) -> f32 {
        match self.effect {
            LightEffect::Glow => {
                let scale = self.level0 - self.level1;
                let phase = time / 2.0 / scale;
                (0.5 - fract(phase)).abs() * 2.0 * scale + self.level1
            },
            LightEffect::Flash => {
                if self.noise((time * 20.0).floor()) < 0.06 { self.level1 } else { self.level0 }
            },
            LightEffect::Flicker => {
                if self.noise((time * 8.0).floor()) < 0.5 { self.level1 } else { self.level0 }
            },
            LightEffect::SlowStrobe => {
                if fract(time + self.sync * 3.5435) > 0.85 { self.level0 } else { self.level1 }
            },
            LightEffect::FastStrobe => {
                if fract(time * 2.0 + self.sync * 3.5435) > 0.7 { self.level0 }
                else { self.level1 }
            },
            LightEffect::Constant => {
                self.level0
            }
        }
    }
}

fn fract(x: f32) -> f32 { x - x.floor() }

fn apply_contrast(level: LightLevel, fake_contrast: FakeContrast) -> LightLevel {
    match fake_contrast {
        FakeContrast::Darken => if level <= 16 { 0 } else { level - 16 },
        FakeContrast::Brighten => if level >= LightLevel::max_value() - 16 {
            LightLevel::max_value()
        } else {
            level + 16
        },
        _ => level,
    }
}
