use math::Vec2f;
use std::mem;
use std::vec::Vec;
use archive::Archive;
use types::{WadLinedef, WadSeg, WadSidedef, WadSubsector, WadThing, WadVertex};
use types::{LightLevel, SectorId, VertexId, WadNode, WadSector};
use util::from_wad_coords;
use error::Result;

const THINGS_OFFSET: usize = 1;
const LINEDEFS_OFFSET: usize = 2;
const SIDEDEFS_OFFSET: usize = 3;
const VERTICES_OFFSET: usize = 4;
const SEGS_OFFSET: usize = 5;
const SSECTORS_OFFSET: usize = 6;
const NODES_OFFSET: usize = 7;
const SECTORS_OFFSET: usize = 8;

pub struct Level {
    pub things: Vec<WadThing>,
    pub linedefs: Vec<WadLinedef>,
    pub sidedefs: Vec<WadSidedef>,
    pub vertices: Vec<WadVertex>,
    pub segs: Vec<WadSeg>,
    pub subsectors: Vec<WadSubsector>,
    pub nodes: Vec<WadNode>,
    pub sectors: Vec<WadSector>,
}

impl Level {
    pub fn from_archive(wad: &Archive, index: usize) -> Result<Level> {
        let name = *wad.level_name(index);
        info!("Reading level data for '{}'...", name);
        let start_index = wad.level_lump_index(index);
        let things = try!(wad.read_lump(start_index + THINGS_OFFSET));
        let linedefs = try!(wad.read_lump(start_index + LINEDEFS_OFFSET));
        let vertices = try!(wad.read_lump(start_index + VERTICES_OFFSET));
        let segs = try!(wad.read_lump(start_index + SEGS_OFFSET));
        let subsectors = try!(wad.read_lump(start_index + SSECTORS_OFFSET));
        let nodes = try!(wad.read_lump(start_index + NODES_OFFSET));
        let sidedefs = try!(wad.read_lump::<WadSidedef>(start_index + SIDEDEFS_OFFSET));
        let sectors = try!(wad.read_lump::<WadSector>(start_index + SECTORS_OFFSET));

        info!("Loaded level '{}':", name);
        info!("    {:4} things", things.len());
        info!("    {:4} linedefs", linedefs.len());
        info!("    {:4} sidedefs", sidedefs.len());
        info!("    {:4} vertices", vertices.len());
        info!("    {:4} segs", segs.len());
        info!("    {:4} subsectors", subsectors.len());
        info!("    {:4} nodes", nodes.len());
        info!("    {:4} sectors", sectors.len());

        Ok(Level {
            things: things,
            linedefs: linedefs,
            sidedefs: sidedefs,
            vertices: vertices,
            segs: segs,
            subsectors: subsectors,
            nodes: nodes,
            sectors: sectors,
        })
    }

    pub fn vertex(&self, id: VertexId) -> Option<Vec2f> {
        self.vertices.get(id as usize).map(|v| from_wad_coords(v.x, v.y))
    }

    pub fn seg_linedef(&self, seg: &WadSeg) -> Option<&WadLinedef> {
        self.linedefs.get(seg.linedef as usize)
    }

    pub fn seg_vertices(&self, seg: &WadSeg) -> Option<(Vec2f, Vec2f)> {
        if let (Some(v1), Some(v2)) = (self.vertex(seg.start_vertex), self.vertex(seg.end_vertex)) {
            Some((v1, v2))
        } else {
            None
        }
    }

    pub fn seg_sidedef(&self, seg: &WadSeg) -> Option<&WadSidedef> {
        self.seg_linedef(seg).and_then(|line| {
            if seg.direction == 0 {
                self.right_sidedef(line)
            } else {
                self.left_sidedef(line)
            }
        })
    }

    pub fn seg_back_sidedef(&self, seg: &WadSeg) -> Option<&WadSidedef> {
        self.seg_linedef(seg).and_then(|line| {
            if seg.direction == 1 {
                self.right_sidedef(line)
            } else {
                self.left_sidedef(line)
            }
        })
    }

    pub fn seg_sector(&self, seg: &WadSeg) -> Option<&WadSector> {
        self.seg_sidedef(seg).and_then(|side| self.sidedef_sector(side))
    }

    pub fn seg_back_sector(&self, seg: &WadSeg) -> Option<&WadSector> {
        self.seg_back_sidedef(seg).and_then(|side| self.sidedef_sector(side))
    }

    pub fn left_sidedef(&self, linedef: &WadLinedef) -> Option<&WadSidedef> {
        match linedef.left_side {
            -1 => None,
            index => self.sidedefs.get(index as usize),
        }
    }

    pub fn right_sidedef(&self, linedef: &WadLinedef) -> Option<&WadSidedef> {
        match linedef.right_side {
            -1 => None,
            index => self.sidedefs.get(index as usize),
        }
    }

    pub fn sidedef_sector(&self, sidedef: &WadSidedef) -> Option<&WadSector> {
        self.sectors.get(sidedef.sector as usize)
    }

    pub fn ssector(&self, index: usize) -> Option<&WadSubsector> {
        self.subsectors.get(index)
    }

    pub fn ssector_segs(&self, ssector: &WadSubsector) -> Option<&[WadSeg]> {
        let start = ssector.first_seg as usize;
        let end = start + ssector.num_segs as usize;
        if end <= self.segs.len() {
            Some(&self.segs[start..end])
        } else {
            None
        }
    }

    pub fn sector_id(&self, sector: &WadSector) -> SectorId {
        let sector_id = (sector as *const _ as usize - self.sectors.as_ptr() as usize) /
                        mem::size_of::<WadSector>();
        assert!(sector_id < self.sectors.len());
        sector_id as SectorId
    }

    pub fn sector_min_light(&self, sector: &WadSector) -> LightLevel {
        let mut min_light = sector.light;
        let sector_id = self.sector_id(sector);
        for line in &self.linedefs {
            let left = match self.left_sidedef(line) {
                Some(l) => l.sector,
                None => continue,
            };
            let right = match self.right_sidedef(line) {
                Some(r) => r.sector,
                None => continue,
            };
            let adjacent_light = if left == sector_id {
                self.sectors.get(right as usize).map(|s| s.light)
            } else if right == sector_id {
                self.sectors.get(left as usize).map(|s| s.light)
            } else {
                continue;
            };
            if let Some(light) = adjacent_light {
                if light < min_light {
                    min_light = light;
                }
            } else {
                warn!("Bad WAD: Cannot access all adjacent sectors to find minimum light.");
            }
        }
        min_light
    }
}
