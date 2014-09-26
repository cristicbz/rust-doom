use std::mem;
use std::slice::raw;
use numvec::{Vec2, Vec2f};
use super::types::{WadCoord, WadInfo, WadName, ChildId};
use std::ascii::ASCII_UPPER_MAP;

pub enum WadType { Initial, Patch }


static IWAD_HEADER: &'static [u8] = b"IWAD";
static PWAD_HEADER: &'static [u8] = b"PWAD";


pub fn wad_type_from_info(wad_info : &WadInfo) -> Option<WadType> {
    let id : &[u8] = &wad_info.identifier;
    match id {
        IWAD_HEADER => Some(Initial),
        PWAD_HEADER => Some(Patch),
        _           => None
    }
}


pub fn read_binary<T : Copy, R : Reader>(reader : &mut R) -> T {
    let mut loaded : T = unsafe { mem::zeroed() };
    let size = mem::size_of::<T>();
    unsafe {
        raw::mut_buf_as_slice(
            &mut loaded as *mut T as *mut u8, size,
            |buf| { reader.read_at_least(size, buf).unwrap() });
    };
    loaded
}


pub fn name_toupper(name: &[u8]) -> Vec<u8> {
    let mut name = name.to_vec();
    for c in name.iter_mut() {
        *c = ASCII_UPPER_MAP[*c as uint];
    }
    name
}

static NUKAGE_FRAMES: &'static [&'static [u8]] =
    &[b"NUKAGE1\0", b"NUKAGE2\0", b"NUKAGE3\0"];
static FWATER_FRAMES: &'static [&'static [u8]] =
    &[b"FWATER1\0", b"FWATER2\0", b"FWATER3\0", b"FWATER4\0"];
static SWATER_FRAMES: &'static [&'static [u8]] =
    &[b"SWATER1\0", b"SWATER2\0", b"SWATER3\0", b"SWATER4\0"];
static LAVA_FRAMES: &'static [&'static [u8]] =
    &[b"LAVA1\0\0\0", b"LAVA2\0\0\0", b"LAVA3\0\0\0", b"LAVA4\0\0\0"];
static BLOOD_FRAMES: &'static [&'static [u8]] =
    &[b"BLOOD1\0\0", b"BLOOD2\0\0", b"BLOOD3\0\0"];
static RROCK05_FRAMES: &'static [&'static [u8]] =
    &[b"RROCK05\0", b"RROCK06\0", b"RROCK07\0", b"RROCK08\0"];
static SLIME01_FRAMES: &'static [&'static [u8]] =
    &[b"SLIME01\0", b"SLIME02\0", b"SLIME03\0", b"SLIME04\0"];
static SLIME05_FRAMES: &'static [&'static [u8]] =
    &[b"SLIME05\0", b"SLIME06\0", b"SLIME07\0", b"SLIME08\0"];
static SLIME09_FRAMES: &'static [&'static [u8]] =
    &[b"SLIME09\0", b"SLIME10\0", b"SLIME11\0", b"SLIME12\0"];
static ANIMATED_FLATS: &'static [&'static [&'static [u8]]] = [
    NUKAGE_FRAMES, FWATER_FRAMES, SWATER_FRAMES, LAVA_FRAMES, BLOOD_FRAMES,
    RROCK05_FRAMES, SLIME01_FRAMES, SLIME05_FRAMES, SLIME09_FRAMES,
];

pub fn flat_frame_names(name: &[u8])
        -> Option<(uint, &'static [&'static [u8]])> {
    for animation in ANIMATED_FLATS.iter() {
        for (i_frame, frame_name) in animation.iter().enumerate() {
            if *frame_name == name {
                return Some((i_frame, *animation))
            }
        }
    }
    None
}


pub fn name_from_str(name: &str) -> [u8, ..8] {
    let bytes = name.as_bytes();
    assert!(bytes.len() <= 8);

    let mut name = [0u8, ..8];
    for i_byte in range(0, bytes.len()) {
        name[i_byte] = bytes[i_byte];
    }
    name
}


pub fn name_eq(name1: &WadName, name2: &[u8]) -> bool {
    for i_byte in range(0, 8) {
        if i_byte == name1.len() { return i_byte == name2.len(); }
        if i_byte == name2.len() { return i_byte == name1.len(); }

        if name1[i_byte] != name2[i_byte] { return false; }
        if name1[i_byte] == 0 { return true; }
    }
    true
}


pub fn name_eq_str(name: &WadName, str_name: &str) -> bool {
    name_eq(name, &name_from_str(str_name))
}


pub fn is_untextured(name: &WadName) -> bool { name[0] == b'-' && name[1] == 0 }
pub fn is_sky_flat(name: &WadName) -> bool { name_eq(name, b"F_SKY1\0\0\0") }

pub fn from_wad_height(x: WadCoord) -> f32 { (x as f32) / 100.0 }


pub fn from_wad_coords(x: WadCoord, y: WadCoord) -> Vec2f {
    Vec2::new(-from_wad_height(x), from_wad_height(y))
}


pub fn parse_child_id(id: ChildId) -> (uint, bool) {
    ((id & 0x7fff) as uint, id & 0x8000 != 0)
}
