use super::archive::Archive;
use super::errors::Result;
use super::types::{LightLevel, SectorId, VertexId, WadNode, WadSector};
use super::types::{WadCoord, WadLinedef, WadSeg, WadSidedef, WadSubsector, WadThing, WadVertex};
use super::util::from_wad_coords;
use log::{error, info};
use math::Pnt2f;
use std::cmp;
use std::mem;
use std::slice::Iter as SliceIter;
use std::vec::Vec;

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
        let lump = wad.level_lump(index)?;
        info!("Reading level data for '{}'...", lump.name());
        let start_index = lump.index();
        let things = wad
            .lump_by_index(start_index + THINGS_OFFSET)?
            .decode_vec()?;
        let linedefs = wad
            .lump_by_index(start_index + LINEDEFS_OFFSET)?
            .decode_vec()?;
        let vertices = wad
            .lump_by_index(start_index + VERTICES_OFFSET)?
            .decode_vec()?;
        let segs = wad.lump_by_index(start_index + SEGS_OFFSET)?.decode_vec()?;
        let subsectors = wad
            .lump_by_index(start_index + SSECTORS_OFFSET)?
            .decode_vec()?;
        let nodes = wad
            .lump_by_index(start_index + NODES_OFFSET)?
            .decode_vec()?;
        let sidedefs = wad
            .lump_by_index(start_index + SIDEDEFS_OFFSET)?
            .decode_vec()?;
        let sectors = wad
            .lump_by_index(start_index + SECTORS_OFFSET)?
            .decode_vec()?;

        info!("Loaded level '{}':", lump.name());
        info!("    {:4} things", things.len());
        info!("    {:4} linedefs", linedefs.len());
        info!("    {:4} sidedefs", sidedefs.len());
        info!("    {:4} vertices", vertices.len());
        info!("    {:4} segs", segs.len());
        info!("    {:4} subsectors", subsectors.len());
        info!("    {:4} nodes", nodes.len());
        info!("    {:4} sectors", sectors.len());

        Ok(Level {
            things,
            linedefs,
            sidedefs,
            vertices,
            segs,
            subsectors,
            nodes,
            sectors,
        })
    }

    pub fn vertex(&self, id: VertexId) -> Option<Pnt2f> {
        self.vertices
            .get(id as usize)
            .map(|v| from_wad_coords(v.x, v.y))
    }

    pub fn seg_linedef(&self, seg: &WadSeg) -> Option<&WadLinedef> {
        self.linedefs.get(seg.linedef as usize)
    }

    pub fn seg_vertices(&self, seg: &WadSeg) -> Option<(Pnt2f, Pnt2f)> {
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
        self.seg_sidedef(seg)
            .and_then(|side| self.sidedef_sector(side))
    }

    pub fn seg_back_sector(&self, seg: &WadSeg) -> Option<&WadSector> {
        self.seg_back_sidedef(seg)
            .and_then(|side| self.sidedef_sector(side))
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

    pub fn ssector(&self, index: usize) -> Option<WadSubsector> {
        self.subsectors.get(index).cloned()
    }

    pub fn ssector_segs(&self, ssector: WadSubsector) -> Option<&[WadSeg]> {
        let start = ssector.first_seg as usize;
        let end = start + ssector.num_segs as usize;
        if end <= self.segs.len() {
            Some(&self.segs[start..end])
        } else {
            None
        }
    }

    pub fn sector_id(&self, sector: &WadSector) -> SectorId {
        let sector_id = (sector as *const _ as usize - self.sectors.as_ptr() as usize)
            / mem::size_of::<WadSector>();
        assert!(sector_id < self.sectors.len());
        sector_id as SectorId
    }

    pub fn adjacent_sectors(&self, sector: &WadSector) -> AdjacentSectorsIter {
        AdjacentSectorsIter {
            level: self,
            sector_id: self.sector_id(sector),
            linedefs: self.linedefs.iter(),
        }
    }

    pub fn sector_min_light(&self, of: &WadSector) -> LightLevel {
        self.adjacent_sectors(of)
            .map(|sector| sector.light)
            .fold(of.light, cmp::min)
    }

    pub fn neighbour_heights(&self, of: &WadSector) -> Option<NeighbourHeights> {
        let of_floor = of.floor_height;
        self.adjacent_sectors(of).fold(None, |heights, sector| {
            let (floor, ceiling) = (sector.floor_height, sector.ceiling_height);
            Some(match heights {
                Some(heights) => NeighbourHeights {
                    lowest_floor: heights.lowest_floor.min(floor),
                    highest_floor: heights.highest_floor.max(floor),
                    lowest_ceiling: heights.lowest_ceiling.min(ceiling),
                    highest_ceiling: heights.highest_ceiling.max(ceiling),

                    next_floor: if floor <= of_floor {
                        heights.next_floor
                    } else if let Some(next_floor) = heights.next_floor {
                        Some(next_floor.min(floor))
                    } else {
                        Some(floor)
                    },
                },
                None => NeighbourHeights {
                    lowest_floor: floor,
                    highest_floor: floor,
                    lowest_ceiling: ceiling,
                    highest_ceiling: ceiling,
                    next_floor: if floor > of_floor { Some(floor) } else { None },
                },
            })
        })
    }
}

#[derive(Copy, Clone, Debug)]
pub struct NeighbourHeights {
    pub lowest_floor: WadCoord,
    pub next_floor: Option<WadCoord>,
    pub highest_floor: WadCoord,
    pub lowest_ceiling: WadCoord,
    pub highest_ceiling: WadCoord,
}

pub struct AdjacentSectorsIter<'a> {
    level: &'a Level,
    sector_id: SectorId,
    linedefs: SliceIter<'a, WadLinedef>,
}

impl<'a> Iterator for AdjacentSectorsIter<'a> {
    type Item = &'a WadSector;

    fn next(&mut self) -> Option<Self::Item> {
        // TODO(cristicbz): Precompute an adjacency matrix for sectors.
        while let Some(line) = self.linedefs.next() {
            let left = match self.level.left_sidedef(line) {
                Some(l) => l.sector,
                None => continue,
            };
            let right = match self.level.right_sidedef(line) {
                Some(r) => r.sector,
                None => continue,
            };
            let adjacent = if left == self.sector_id {
                self.level.sectors.get(right as usize)
            } else if right == self.sector_id {
                self.level.sectors.get(left as usize)
            } else {
                continue;
            };
            if adjacent.is_some() {
                return adjacent;
            } else {
                error!("Bad WAD: Cannot access all adjacent sectors to find minimum light.");
            }
        }
        None
    }
}
