use numvec::Vec2f;
use std::mem;
use std::vec::Vec;
use super::archive::Archive;
use super::types::{WadThing, WadLinedef, WadSidedef, WadVertex, WadSeg,
                   WadSubsector, WadNode, WadSector, VertexId, LightLevel,
                   SectorId};
use super::util::from_wad_coords;


const THINGS_OFFSET: uint = 1;
const LINEDEFS_OFFSET: uint = 2;
const SIDEDEFS_OFFSET: uint = 3;
const VERTICES_OFFSET: uint = 4;
const SEGS_OFFSET: uint = 5;
const SSECTORS_OFFSET: uint = 6;
const NODES_OFFSET: uint = 7;
const SECTORS_OFFSET: uint = 8;


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
    pub fn from_archive(wad: &mut Archive, index: uint) -> Level {
        let name = *wad.get_level_name(index);
        info!("Reading level data for '{}'...", name);
        let start_index = wad.get_level_lump_index(index);
        let things = wad.read_lump(start_index + THINGS_OFFSET);
        let linedefs = wad.read_lump(start_index + LINEDEFS_OFFSET);
        let vertices = wad.read_lump(start_index + VERTICES_OFFSET);
        let segs = wad.read_lump(start_index + SEGS_OFFSET);
        let subsectors = wad.read_lump(start_index + SSECTORS_OFFSET);
        let nodes = wad.read_lump(start_index + NODES_OFFSET);

        let mut sidedefs = wad.read_lump::<WadSidedef>(
                start_index + SIDEDEFS_OFFSET);
        for side in sidedefs.iter_mut() {
            side.upper_texture.canonicalise();
            side.lower_texture.canonicalise();
            side.middle_texture.canonicalise();
        }
        let sidedefs = sidedefs;

        let mut sectors = wad.read_lump::<WadSector>(
                start_index + SECTORS_OFFSET);
        for sector in sectors.iter_mut() {
            sector.floor_texture.canonicalise();
            sector.ceiling_texture.canonicalise();
        }
        let sectors = sectors;

        info!("Loaded level '{}':", name);
        info!("    {:4} things", things.len())
        info!("    {:4} linedefs", linedefs.len())
        info!("    {:4} sidedefs", sidedefs.len())
        info!("    {:4} vertices", vertices.len())
        info!("    {:4} segs", segs.len())
        info!("    {:4} subsectors", subsectors.len())
        info!("    {:4} nodes", nodes.len())
        info!("    {:4} sectors", sectors.len())

        Level {
            things: things,
            linedefs: linedefs,
            sidedefs: sidedefs,
            vertices: vertices,
            segs: segs,
            subsectors: subsectors,
            nodes: nodes,
            sectors: sectors,
        }
    }

    pub fn vertex(&self, id: VertexId) -> Vec2f {
        from_wad_coords(self.vertices[id as uint].x,
                        self.vertices[id as uint].y)
    }

    pub fn seg_linedef(&self, seg: &WadSeg) -> &WadLinedef {
        &self.linedefs[seg.linedef as uint]
    }

    pub fn seg_vertices(&self, seg: &WadSeg) -> (Vec2f, Vec2f) {
        (self.vertex(seg.start_vertex), self.vertex(seg.end_vertex))
    }

    pub fn seg_sidedef(&self, seg: &WadSeg) -> &WadSidedef {
        let line = self.seg_linedef(seg);
        if seg.direction == 0 { self.right_sidedef(line).unwrap() }
        else { self.left_sidedef(line).unwrap() }
    }

    pub fn seg_back_sidedef(&self, seg: &WadSeg) -> Option<&WadSidedef> {
        let line = self.seg_linedef(seg);
        if seg.direction == 1 { self.right_sidedef(line) }
        else { self.left_sidedef(line) }
    }

    pub fn seg_sector(&self, seg: &WadSeg) -> &WadSector {
        self.sidedef_sector(self.seg_sidedef(seg))
    }

    pub fn seg_back_sector(&self, seg: &WadSeg) -> Option<&WadSector> {
        self.seg_back_sidedef(seg).map(|s| self.sidedef_sector(s))
    }

    pub fn left_sidedef(&self, linedef: &WadLinedef) -> Option<&WadSidedef> {
        match linedef.left_side {
            -1 => None,
            index => Some(&self.sidedefs[index as uint])
        }
    }

    pub fn right_sidedef(&self, linedef: &WadLinedef) -> Option<&WadSidedef> {
        match linedef.right_side {
            -1 => None,
            index => Some(&self.sidedefs[index as uint])
        }
    }

    pub fn sidedef_sector(&self, sidedef: &WadSidedef) -> &WadSector {
        &self.sectors[sidedef.sector as uint]
    }

    pub fn ssector_segs(&self, ssector: &WadSubsector) -> &[WadSeg] {
        let start = ssector.first_seg as uint;
        let end = start + ssector.num_segs as uint;
        self.segs[start .. end]
    }

    pub fn sector_id(&self, sector: &WadSector) -> SectorId {
        let sector_id =
            (sector as *const _ as uint - self.sectors.as_ptr() as uint) /
            mem::size_of::<WadSector>();
        assert!(sector_id < self.sectors.len());
        return sector_id as SectorId;
    }

    pub fn sector_min_light(&self, sector: &WadSector) -> LightLevel {
        let mut min_light = sector.light;
        let sector_id = self.sector_id(sector);
        for line in self.linedefs.iter() {
            let (left, right) =
                (match self.left_sidedef(line) {
                    Some(l) => l.sector, None => continue,
                 }, match self.right_sidedef(line) {
                    Some(r) => r.sector, None => continue,
                 });
            let adjacent_light = if left == sector_id {
                self.sectors[right as uint].light
            } else if right == sector_id {
                self.sectors[left as uint].light
            } else {
                continue;
            };
            if adjacent_light < min_light { min_light = adjacent_light; }
        }
        min_light
    }
}
