use numvec::Vec2f;
use std::str;
use std::vec::Vec;
use super::archive::Archive;
use super::types::{WadThing, WadLinedef, WadSidedef, WadVertex, WadSeg,
                   WadSubsector, WadNode, WadSector, VertexId, LevelName};
use super::util::from_wad_coords;


static THINGS_OFFSET: uint = 1;
static LINEDEFS_OFFSET: uint = 2;
static SIDEDEFS_OFFSET: uint = 3;
static VERTICES_OFFSET: uint = 4;
static SEGS_OFFSET: uint = 5;
static SSECTORS_OFFSET: uint = 6;
static NODES_OFFSET: uint = 7;
static SECTORS_OFFSET: uint = 8;


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
    pub fn from_archive(wad: &mut Archive, name: &LevelName) -> Level {
        info!("Reading level data for '{}'...", str::from_utf8(name).unwrap());
        let start_index = wad.get_lump_index(name).expect("No such level.");
        let things = wad.read_lump(start_index + THINGS_OFFSET);
        let linedefs = wad.read_lump(start_index + LINEDEFS_OFFSET);
        let sidedefs = wad.read_lump(start_index + SIDEDEFS_OFFSET);
        let vertices = wad.read_lump(start_index + VERTICES_OFFSET);
        let segs = wad.read_lump(start_index + SEGS_OFFSET);
        let subsectors = wad.read_lump(start_index + SSECTORS_OFFSET);
        let nodes = wad.read_lump(start_index + NODES_OFFSET);
        let sectors = wad.read_lump(start_index + SECTORS_OFFSET);

        info!("Loaded level '{}':", str::from_utf8(name).unwrap());
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

    pub fn seg_linedef<'a>(&'a self, seg: &WadSeg) -> &'a WadLinedef {
        &self.linedefs[seg.linedef as uint]
    }

    pub fn seg_vertices(&self, seg: &WadSeg) -> (Vec2f, Vec2f) {
        (self.vertex(seg.start_vertex), self.vertex(seg.end_vertex))
    }

    pub fn left_sidedef<'a>(&'a self, linedef: &WadLinedef)
            -> &'a WadSidedef {
        &self.sidedefs[linedef.left_side as uint]
    }

    pub fn right_sidedef<'a>(&'a self, linedef: &WadLinedef)
            -> &'a WadSidedef {
        &self.sidedefs[linedef.right_side as uint]
    }

    pub fn sidedef_sector<'a>(&'a self, sidedef: &WadSidedef) -> &'a WadSector {
        &self.sectors[sidedef.sector as uint]
    }

    pub fn ssector_segs<'a>(&'a self, ssector: &WadSubsector) -> &'a [WadSeg] {
        self.segs.slice(ssector.first_seg as uint,
                        (ssector.first_seg as uint + ssector.num_segs as uint))
    }
}
