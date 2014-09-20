use std::mem;
use std::slice::raw;
use numvec::{Vec2, Vec2f};
use super::types::{WadCoord, WadInfo, WadName, ChildId};

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
pub fn is_sky_texture(_name: &WadName) -> bool {
    false
}

pub fn from_wad_height(x: WadCoord) -> f32 { (x as f32) / 100.0 }


pub fn from_wad_coords(x: WadCoord, y: WadCoord) -> Vec2f {
    Vec2::new(-from_wad_height(x), from_wad_height(y))
}


pub fn parse_child_id(id: ChildId) -> (uint, bool) {
    ((id & 0x7fff) as uint, id & 0x8000 != 0)
}
