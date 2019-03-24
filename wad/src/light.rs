use super::level::Level;
use super::types::{LightLevel, SectorType, WadSector};
use std::f32::EPSILON;

#[derive(PartialEq, Clone)]
pub struct LightInfo {
    pub level: f32,
    pub effect: Option<LightEffect>,
}

#[derive(PartialEq, Clone)]
pub struct LightEffect {
    pub alt_level: f32,
    pub speed: f32,
    pub duration: f32,
    pub sync: f32,
    pub kind: LightEffectKind,
}

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum LightEffectKind {
    Glow,
    Random,
    Alternate,
}

pub fn new_light(level: &Level, sector: &WadSector) -> LightInfo {
    let base_level = light_to_f32(sector.light);
    let alt_level = match sector.sector_type {
        FLASH | FAST_STROBE_1 | FAST_STROBE_2 | FAST_STROBE_SYNC | SLOW_STROBE
        | SLOW_STROBE_SYNC | GLOW | FLICKER => {
            let alt_level = light_to_f32(level.sector_min_light(sector));
            if (alt_level - base_level).abs() < EPSILON {
                return LightInfo {
                    level: base_level,
                    effect: None,
                };
            } else {
                alt_level
            }
        }
        _ => {
            return LightInfo {
                level: base_level,
                effect: None,
            };
        }
    };
    let sync = match sector.sector_type {
        SLOW_STROBE_SYNC | FAST_STROBE_SYNC | GLOW => 0.0,
        _ => id_to_sync(level.sector_id(sector)),
    };
    let (kind, speed, duration) = match sector.sector_type {
        FLASH => (LightEffectKind::Random, FLASH_SPEED, FLASH_DURATION),
        FLICKER => (LightEffectKind::Random, FLICKER_SPEED, FLICKER_DURATION),
        SLOW_STROBE | SLOW_STROBE_SYNC => (
            LightEffectKind::Alternate,
            SLOW_STROBE_SPEED,
            SLOW_STROBE_DURATION,
        ),
        FAST_STROBE_1 | FAST_STROBE_2 | FAST_STROBE_SYNC => (
            LightEffectKind::Alternate,
            FAST_STROBE_SPEED,
            FAST_STROBE_DURATION,
        ),
        GLOW => (LightEffectKind::Glow, GLOW_SPEED, 0.0),
        _ => unreachable!(),
    };
    LightInfo {
        level: base_level,
        effect: Some(LightEffect {
            alt_level,
            kind,
            speed,
            duration,
            sync,
        }),
    }
}

#[inline]
pub fn with_contrast(light_info: &LightInfo, contrast: Contrast) -> LightInfo {
    let contrast = match contrast {
        Contrast::Darken => -2.0 / 31.0,
        Contrast::Brighten => 2.0 / 31.0,
    };
    LightInfo {
        level: clamp(light_info.level + contrast),
        ..light_info.clone()
    }
}

fn clamp(level: f32) -> f32 {
    if level > 1.0 {
        1.0
    } else if level < 0.0 {
        0.0
    } else {
        level
    }
}

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum Contrast {
    Darken,
    Brighten,
}

fn id_to_sync(id: u16) -> f32 {
    ((u64::from(id) * 1_664_525 + 1_013_904_223) & 0xffff) as f32 / 15.0
}

fn light_to_f32(level: LightLevel) -> f32 {
    f32::from(level >> 3) / 31.0
}

const FLASH_SPEED: f32 = 20.0;
const FLASH_DURATION: f32 = 0.06;
const FLICKER_SPEED: f32 = 8.0;
const FLICKER_DURATION: f32 = 0.5;
const SLOW_STROBE_SPEED: f32 = 1.0;
const SLOW_STROBE_DURATION: f32 = 0.85;
const FAST_STROBE_SPEED: f32 = 2.0;
const FAST_STROBE_DURATION: f32 = 0.7;
const GLOW_SPEED: f32 = 0.5;

const FLASH: SectorType = 1;
const FAST_STROBE_1: SectorType = 2;
const FAST_STROBE_2: SectorType = 4;
const FAST_STROBE_SYNC: SectorType = 13;
const SLOW_STROBE: SectorType = 3;
const SLOW_STROBE_SYNC: SectorType = 12;
const GLOW: SectorType = 8;
const FLICKER: SectorType = 17;
