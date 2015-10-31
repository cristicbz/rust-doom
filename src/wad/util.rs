use math::{Vec2, Vec2f};
use types::{ChildId, WadCoord, WadInfo, WadName};

#[derive(Copy, Clone)]
pub enum WadType {
    Initial,
    Patch,
}

const IWAD_HEADER: &'static [u8] = b"IWAD";
const PWAD_HEADER: &'static [u8] = b"PWAD";

pub fn wad_type_from_info(wad_info: &WadInfo) -> Option<WadType> {
    let id = &wad_info.identifier;
    if id == IWAD_HEADER {
        Some(WadType::Initial)
    } else if id == PWAD_HEADER {
        Some(WadType::Patch)
    } else {
        None
    }
}

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
