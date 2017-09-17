use super::types::{ChildId, WadCoord, WadName};
use math::{Vec2, Vec2f};

pub fn is_untextured(name: &WadName) -> bool {
    name[0] == b'-' && name[1] == b'\0'
}

pub fn is_sky_flat(name: &WadName) -> bool {
    name == b"F_SKY1\0\0"
}

pub fn from_wad_height(x: WadCoord) -> f32 {
    (x as f32) / 100.0
}

pub fn from_wad_coords(x: WadCoord, y: WadCoord) -> Vec2f {
    Vec2::new(-from_wad_height(x), from_wad_height(y))
}

pub fn parse_child_id(id: ChildId) -> (usize, bool) {
    ((id & 0x7fff) as usize, id & 0x8000 != 0)
}
