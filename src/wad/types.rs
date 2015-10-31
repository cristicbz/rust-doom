use error::Result;
use read::{WadRead, WadReadFrom};
use std::io::Read;

pub use super::name::WadName;

pub type LightLevel = i16;
pub type LinedefFlags = u16;
pub type LinedefType = u16;
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


#[derive(Copy, Clone)]
pub struct WadInfo {
    pub identifier: [u8; 4],
    pub num_lumps: i32,
    pub info_table_offset: i32,
}

impl WadReadFrom for WadInfo {
    fn wad_read_from<R: Read>(reader: &mut R) -> Result<Self> {
        let identifier = try!(reader.wad_read::<u32>());
        Ok(WadInfo {
            identifier: [(identifier & 0xff) as u8,
                         ((identifier >> 8) & 0xff) as u8,
                         ((identifier >> 16) & 0xff) as u8,
                         ((identifier >> 24) & 0xff) as u8],
            num_lumps: try!(reader.wad_read()),
            info_table_offset: try!(reader.wad_read()),
        })
    }
}


#[derive(Copy, Clone)]
pub struct WadLump {
    pub file_pos: i32,
    pub size: i32,
    pub name: WadName,
}

impl WadReadFrom for WadLump {
    fn wad_read_from<R: Read>(reader: &mut R) -> Result<Self> {
        Ok(WadLump {
            file_pos: try!(reader.wad_read()),
            size: try!(reader.wad_read()),
            name: try!(reader.wad_read()),
        })
    }
}


#[derive(Copy, Clone)]
pub struct WadThing {
    pub x: WadCoord,
    pub y: WadCoord,
    pub angle: WadCoord,
    pub thing_type: ThingType,
    pub flags: ThingFlags,
}

impl WadReadFrom for WadThing {
    fn wad_read_from<R: Read>(reader: &mut R) -> Result<Self> {
        Ok(WadThing {
            x: try!(reader.wad_read()),
            y: try!(reader.wad_read()),
            angle: try!(reader.wad_read()),
            thing_type: try!(reader.wad_read()),
            flags: try!(reader.wad_read()),
        })
    }
}


#[derive(Copy, Clone)]
pub struct WadVertex {
    pub x: WadCoord,
    pub y: WadCoord,
}

impl WadReadFrom for WadVertex {
    fn wad_read_from<R: Read>(reader: &mut R) -> Result<Self> {
        Ok(WadVertex {
            x: try!(reader.wad_read()),
            y: try!(reader.wad_read()),
        })
    }
}


#[derive(Copy, Clone)]
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

impl WadReadFrom for WadLinedef {
    fn wad_read_from<R: Read>(reader: &mut R) -> Result<Self> {
        Ok(WadLinedef {
            start_vertex: try!(reader.wad_read()),
            end_vertex: try!(reader.wad_read()),
            flags: try!(reader.wad_read()),
            special_type: try!(reader.wad_read()),
            sector_tag: try!(reader.wad_read()),
            right_side: try!(reader.wad_read()),
            left_side: try!(reader.wad_read()),
        })
    }
}


#[derive(Copy, Clone)]
pub struct WadSidedef {
    pub x_offset: WadCoord,
    pub y_offset: WadCoord,
    pub upper_texture: WadName,
    pub lower_texture: WadName,
    pub middle_texture: WadName,
    pub sector: SectorId,
}

impl WadReadFrom for WadSidedef {
    fn wad_read_from<R: Read>(reader: &mut R) -> Result<Self> {
        Ok(WadSidedef {
            x_offset: try!(reader.wad_read()),
            y_offset: try!(reader.wad_read()),
            upper_texture: try!(reader.wad_read()),
            lower_texture: try!(reader.wad_read()),
            middle_texture: try!(reader.wad_read()),
            sector: try!(reader.wad_read()),
        })
    }
}


#[derive(Copy, Clone)]
pub struct WadSector {
    pub floor_height: WadCoord,
    pub ceiling_height: WadCoord,
    pub floor_texture: WadName,
    pub ceiling_texture: WadName,
    pub light: LightLevel,
    pub sector_type: SectorType,
    pub tag: SectorTag,
}

impl WadReadFrom for WadSector {
    fn wad_read_from<R: Read>(reader: &mut R) -> Result<Self> {
        Ok(WadSector {
            floor_height: try!(reader.wad_read()),
            ceiling_height: try!(reader.wad_read()),
            floor_texture: try!(reader.wad_read()),
            ceiling_texture: try!(reader.wad_read()),
            light: try!(reader.wad_read()),
            sector_type: try!(reader.wad_read()),
            tag: try!(reader.wad_read()),
        })
    }
}


#[derive(Copy, Clone)]
pub struct WadSubsector {
    pub num_segs: u16,
    pub first_seg: SegId,
}

impl WadReadFrom for WadSubsector {
    fn wad_read_from<R: Read>(reader: &mut R) -> Result<Self> {
        Ok(WadSubsector {
            num_segs: try!(reader.wad_read()),
            first_seg: try!(reader.wad_read()),
        })
    }
}


#[derive(Copy, Clone)]
pub struct WadSeg {
    pub start_vertex: VertexId,
    pub end_vertex: VertexId,
    pub angle: u16,
    pub linedef: LinedefId,
    pub direction: u16,
    pub offset: u16,
}

impl WadReadFrom for WadSeg {
    fn wad_read_from<R: Read>(reader: &mut R) -> Result<Self> {
        Ok(WadSeg {
            start_vertex: try!(reader.wad_read()),
            end_vertex: try!(reader.wad_read()),
            angle: try!(reader.wad_read()),
            linedef: try!(reader.wad_read()),
            direction: try!(reader.wad_read()),
            offset: try!(reader.wad_read()),
        })
    }
}


#[derive(Copy, Clone)]
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

impl WadReadFrom for WadNode {
    fn wad_read_from<R: Read>(reader: &mut R) -> Result<Self> {
        Ok(WadNode {
            line_x: try!(reader.wad_read()),
            line_y: try!(reader.wad_read()),
            step_x: try!(reader.wad_read()),
            step_y: try!(reader.wad_read()),
            right_y_max: try!(reader.wad_read()),
            right_y_min: try!(reader.wad_read()),
            right_x_max: try!(reader.wad_read()),
            right_x_min: try!(reader.wad_read()),
            left_y_max: try!(reader.wad_read()),
            left_y_min: try!(reader.wad_read()),
            left_x_max: try!(reader.wad_read()),
            left_x_min: try!(reader.wad_read()),
            right: try!(reader.wad_read()),
            left: try!(reader.wad_read()),
        })
    }
}


#[derive(Copy, Clone)]
pub struct WadTextureHeader {
    pub name: WadName,
    pub masked: u32,
    pub width: u16,
    pub height: u16,
    pub column_directory: u32,
    pub num_patches: u16,
}

impl WadReadFrom for WadTextureHeader {
    fn wad_read_from<R: Read>(reader: &mut R) -> Result<Self> {
        Ok(WadTextureHeader {
            name: try!(reader.wad_read()),
            masked: try!(reader.wad_read()),
            width: try!(reader.wad_read()),
            height: try!(reader.wad_read()),
            column_directory: try!(reader.wad_read()),
            num_patches: try!(reader.wad_read()),
        })
    }
}


#[derive(Copy, Clone)]
pub struct WadTexturePatchRef {
    pub origin_x: i16,
    pub origin_y: i16,
    pub patch: u16,
    pub stepdir: u16,
    pub colormap: u16,
}

impl WadReadFrom for WadTexturePatchRef {
    fn wad_read_from<R: Read>(reader: &mut R) -> Result<Self> {
        Ok(WadTexturePatchRef {
            origin_x: try!(reader.wad_read()),
            origin_y: try!(reader.wad_read()),
            patch: try!(reader.wad_read()),
            stepdir: try!(reader.wad_read()),
            colormap: try!(reader.wad_read()),
        })
    }
}
