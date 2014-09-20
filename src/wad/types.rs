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
pub type WadName = [u8, ..8];


#[repr(C)]
#[repr(packed)]
pub struct WadInfo {
    pub identifier        : [u8, ..4],
    pub num_lumps         : i32,
    pub info_table_offset : i32,
}


#[repr(C)]
#[repr(packed)]
pub struct WadLump {
    pub file_pos : i32,
    pub size     : i32,
    pub name     : [u8, ..8],
}


#[packed]
#[repr(C)]
pub struct WadThing {
    pub x: WadCoord,
    pub y: WadCoord,
    pub angle: WadCoord,
    pub thing_type: ThingType,
    pub flags: ThingFlags,
}


#[packed]
#[repr(C)]
pub struct WadVertex {
    pub x: WadCoord,
    pub y: WadCoord,
}


#[packed]
#[repr(C)]
pub struct WadLinedef {
    pub start_vertex: VertexId,
    pub end_vertex: VertexId,
    pub flags: LinedefFlags,
    pub special_type: SpecialType,
    pub sector_tag: SectorTag,
    pub right_side: SidedefId,
    pub left_side: SidedefId,
}


#[packed]
#[repr(C)]
pub struct WadSidedef {
    pub x_offset: WadCoord,
    pub y_offset: WadCoord,
    pub upper_texture: WadName,
    pub lower_texture: WadName,
    pub middle_texture: WadName,
    pub sector: SectorId,
}


#[packed]
#[repr(C)]
pub struct WadSector {
    pub floor_height: WadCoord,
    pub ceiling_height: WadCoord,
    pub floor_texture: WadName,
    pub ceiling_texture: WadName,
    pub light: LightLevel,
    pub sector_type: SectorType,
    pub tag: SectorTag,
}


#[packed]
#[repr(C)]
pub struct WadSubsector {
    pub num_segs: u16,
    pub first_seg: SegId,
}


#[packed]
#[repr(C)]
pub struct WadSeg {
    pub start_vertex: VertexId,
    pub end_vertex: VertexId,
    pub angle: u16,
    pub linedef: LinedefId,
    pub direction: u16,
    pub offset: u16,
}


#[packed]
#[repr(C)]
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
    pub left: ChildId
}


#[packed]
#[repr(C)]
pub struct WadTextureHeader {
    pub name: WadName,
    pub masked: u32,
    pub width: u16,
    pub height: u16,
    pub column_directory: u32,
    pub num_patches: u16
}


#[packed]
#[repr(C)]
pub struct WadTexturePatchRef {
    pub origin_x: u16,
    pub origin_y: u16,
    pub patch: u16,
    pub stepdir: u16,
    pub colormap: u16,
}
