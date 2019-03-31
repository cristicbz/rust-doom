pub use super::name::WadName;
use serde::Deserialize;

pub type LightLevel = i16;
pub type LinedefFlags = u16;
pub type SectorId = u16;
pub type SectorTag = u16;
pub type SectorType = u16;
pub type SidedefId = i16;
pub type SpecialType = u16;
pub type ThingFlags = u16;
pub type ThingType = u16;
pub type VertexId = u16;
pub type WadCoord = i16;
pub type SegId = u16;
pub type LinedefId = u16;
pub type ChildId = u16;

#[derive(Copy, Clone, Deserialize)]
pub struct WadInfo {
    pub identifier: [u8; 4],
    pub num_lumps: i32,
    pub info_table_offset: i32,
}

#[derive(Copy, Clone, Deserialize)]
pub struct WadLump {
    pub file_pos: i32,
    pub size: i32,
    pub name: WadName,
}

#[derive(Copy, Clone, Deserialize)]
pub struct WadThing {
    pub x: WadCoord,
    pub y: WadCoord,
    pub angle: WadCoord,
    pub thing_type: ThingType,
    pub flags: ThingFlags,
}

#[derive(Copy, Clone, Deserialize)]
pub struct WadVertex {
    pub x: WadCoord,
    pub y: WadCoord,
}

#[derive(Copy, Clone, Deserialize)]
pub struct WadLinedef {
    pub start_vertex: VertexId,
    pub end_vertex: VertexId,
    pub flags: LinedefFlags,
    pub special_type: SpecialType,
    pub sector_tag: SectorTag,
    pub right_side: SidedefId,
    pub left_side: SidedefId,
}

impl WadLinedef {
    pub fn impassable(&self) -> bool {
        self.flags & 0x0001 != 0
    }

    pub fn blocks_monsters(&self) -> bool {
        self.flags & 0x0002 != 0
    }

    pub fn is_two_sided(&self) -> bool {
        self.flags & 0x0004 != 0
    }

    pub fn upper_unpegged(&self) -> bool {
        self.flags & 0x0008 != 0
    }

    pub fn lower_unpegged(&self) -> bool {
        self.flags & 0x0010 != 0
    }

    pub fn secret(&self) -> bool {
        self.flags & 0x0020 != 0
    }

    pub fn blocks_sound(&self) -> bool {
        self.flags & 0x0040 != 0
    }

    pub fn always_shown_on_map(&self) -> bool {
        self.flags & 0x0080 != 0
    }

    pub fn never_shown_on_map(&self) -> bool {
        self.flags & 0x0100 != 0
    }
}

#[derive(Copy, Clone, Deserialize)]
pub struct WadSidedef {
    pub x_offset: WadCoord,
    pub y_offset: WadCoord,
    pub upper_texture: WadName,
    pub lower_texture: WadName,
    pub middle_texture: WadName,
    pub sector: SectorId,
}

#[derive(Copy, Clone, Deserialize)]
pub struct WadSector {
    pub floor_height: WadCoord,
    pub ceiling_height: WadCoord,
    pub floor_texture: WadName,
    pub ceiling_texture: WadName,
    pub light: LightLevel,
    pub sector_type: SectorType,
    pub tag: SectorTag,
}

#[derive(Copy, Clone, Deserialize)]
pub struct WadSubsector {
    pub num_segs: u16,
    pub first_seg: SegId,
}

#[derive(Copy, Clone, Deserialize)]
pub struct WadSeg {
    pub start_vertex: VertexId,
    pub end_vertex: VertexId,
    pub angle: u16,
    pub linedef: LinedefId,
    pub direction: u16,
    pub offset: u16,
}

#[derive(Copy, Clone, Deserialize)]
pub struct WadNode {
    pub line_x: WadCoord,
    pub line_y: WadCoord,
    pub step_x: WadCoord,
    pub step_y: WadCoord,
    pub right_y_max: WadCoord,
    pub right_y_min: WadCoord,
    pub right_x_max: WadCoord,
    pub right_x_min: WadCoord,
    pub left_y_max: WadCoord,
    pub left_y_min: WadCoord,
    pub left_x_max: WadCoord,
    pub left_x_min: WadCoord,
    pub right: ChildId,
    pub left: ChildId,
}

#[derive(Copy, Clone, Deserialize)]
pub struct WadTextureHeader {
    pub name: WadName,
    pub masked: u32,
    pub width: u16,
    pub height: u16,
    pub column_directory: u32,
    pub num_patches: u16,
}

#[derive(Copy, Clone, Deserialize)]
pub struct WadTexturePatchRef {
    pub origin_x: i16,
    pub origin_y: i16,
    pub patch: u16,
    pub stepdir: u16,
    pub colormap: u16,
}

pub const PALETTE_SIZE: usize = 256 * 3;
pub const COLORMAP_SIZE: usize = 256;

pub struct Palette(pub [u8; PALETTE_SIZE]);
impl Default for Palette {
    fn default() -> Self {
        Palette([0u8; PALETTE_SIZE])
    }
}
impl AsMut<[u8]> for Palette {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

pub struct Colormap(pub [u8; COLORMAP_SIZE]);
impl Default for Colormap {
    fn default() -> Self {
        Colormap([0u8; COLORMAP_SIZE])
    }
}
impl AsMut<[u8]> for Colormap {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}
