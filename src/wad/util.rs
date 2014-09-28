use std::mem;
use std::slice::raw;
use numvec::{Vec2, Vec2f};
use super::types::{WadCoord, WadInfo, WadName, ChildId, WadNameCast};

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


static NUKAGE_FRAMES: &'static [&'static [u8]] =
    &[b"NUKAGE1", b"NUKAGE2", b"NUKAGE3"];
static FWATER_FRAMES: &'static [&'static [u8]] =
    &[b"FWATER1", b"FWATER2", b"FWATER3", b"FWATER4"];
static SWATER_FRAMES: &'static [&'static [u8]] =
    &[b"SWATER1", b"SWATER2", b"SWATER3", b"SWATER4"];
static LAVA_FRAMES: &'static [&'static [u8]] =
    &[b"LAVA1", b"LAVA2", b"LAVA3", b"LAVA4"];
static BLOOD_FRAMES: &'static [&'static [u8]] =
    &[b"BLOOD1", b"BLOOD2", b"BLOOD3"];
static RROCK05_FRAMES: &'static [&'static [u8]] =
    &[b"RROCK05", b"RROCK06", b"RROCK07", b"RROCK08"];
static SLIME01_FRAMES: &'static [&'static [u8]] =
    &[b"SLIME01", b"SLIME02", b"SLIME03", b"SLIME04"];
static SLIME05_FRAMES: &'static [&'static [u8]] =
    &[b"SLIME05", b"SLIME06", b"SLIME07", b"SLIME08"];
static SLIME09_FRAMES: &'static [&'static [u8]] =
    &[b"SLIME09", b"SLIME10", b"SLIME11", b"SLIME12"];
static ANIMATED_FLATS: &'static [&'static [&'static [u8]]] = [
    NUKAGE_FRAMES, FWATER_FRAMES, SWATER_FRAMES, LAVA_FRAMES, BLOOD_FRAMES,
    RROCK05_FRAMES, SLIME01_FRAMES, SLIME05_FRAMES, SLIME09_FRAMES,
];

pub fn flat_frame_names(name: &WadName) -> Option<&'static [&'static [u8]]> {
    for animation in ANIMATED_FLATS.iter() {
        for frame_name in animation.iter() {
            if &frame_name.to_wad_name() == name {
                return Some(*animation);
            }
        }
    }
    None
}


static BLODGR1_FRAMES: &'static [&'static [u8]] =
    &[b"BLODGR1", b"BLODGR2", b"BLODGR3", b"BLODGR4"];
static BLODRIP1_FRAMES: &'static [&'static [u8]] =
    &[b"BLODRIP1", b"BLODRIP2", b"BLODRIP3", b"BLODRIP4"];
static FIREBLU1_FRAMES: &'static [&'static [u8]] =
    &[b"FIREBLU1", b"FIREBLU2"];
static FIRLAV3_FRAMES: &'static [&'static [u8]] =
    &[b"FIRELAV3", b"FIRELAVA"];
static FIREMAG1_FRAMES: &'static [&'static [u8]] =
    &[b"FIREMAG1", b"FIREMAG2", b"FIREMAG3"];
static FIREWALA_FRAMES: &'static [&'static [u8]] =
    &[b"FIREWALA", b"FIREWALB", b"FIREWALL"];
static GSTFONT1_FRAMES: &'static [&'static [u8]] =
    &[b"GSTFONT1", b"GSTFONT2", b"GSTFONT3"];
static ROCKRED1_FRAMES: &'static [&'static [u8]] =
    &[b"ROCKRED1", b"ROCKRED2", b"ROCKRED3"];
static SLADRIP1_FRAMES: &'static [&'static [u8]] =
    &[b"SLADRIP1", b"SLADRIP2", b"SLADRIP3"];
static BFALL1_FRAMES: &'static [&'static [u8]] =
    &[b"BFALL1", b"BFALL2", b"BFALL3", b"BFALL4"];
static SFALL1_FRAMES: &'static [&'static [u8]] =
    &[b"SFALL1", b"SFALL2", b"SFALL3", b"SFALL4"];
static WFALL1_FRAMES: &'static [&'static [u8]] =
    &[b"WFALL1", b"WFALL2", b"WFALL3", b"WFALL4"];
static DBRAIN1_FRAMES: &'static [&'static [u8]] =
    &[b"DBRAIN1", b"DBRAIN2", b"DBRAIN3",  b"DBRAIN4"];
static ANIMATED_WALLS: &'static [&'static [&'static [u8]]] = [
    BLODGR1_FRAMES, BLODRIP1_FRAMES, FIREBLU1_FRAMES, FIRLAV3_FRAMES,
    FIREMAG1_FRAMES, FIREWALA_FRAMES, GSTFONT1_FRAMES, ROCKRED1_FRAMES,
    SLADRIP1_FRAMES, BFALL1_FRAMES, SFALL1_FRAMES, WFALL1_FRAMES,
    DBRAIN1_FRAMES];


pub fn wall_frame_names(name: &WadName) -> Option<&'static [&'static [u8]]> {
    for animation in ANIMATED_WALLS.iter() {
        for frame_name in animation.iter() {
            if &frame_name.to_wad_name() == name {
                return Some(*animation);
            }
        }
    }
    None
}

pub fn is_untextured(name: &WadName) -> bool {
    let bytes = name.as_bytes();
    bytes[0] == b'-' && bytes[1] == 0
}

pub fn is_sky_flat(name: &WadName) -> bool { name == &b"F_SKY1".to_wad_name() }

pub fn from_wad_height(x: WadCoord) -> f32 { (x as f32) / 100.0 }


pub fn from_wad_coords(x: WadCoord, y: WadCoord) -> Vec2f {
    Vec2::new(-from_wad_height(x), from_wad_height(y))
}


pub fn parse_child_id(id: ChildId) -> (uint, bool) {
    ((id & 0x7fff) as uint, id & 0x8000 != 0)
}
