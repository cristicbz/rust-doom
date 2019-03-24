use super::level::{Level, NeighbourHeights};
use super::light::{self, Contrast, LightInfo};
use super::meta::{
    ExitEffectDef, HeightDef, HeightEffectDef, HeightRef, MoveEffectDef, TriggerType, WadMetadata,
};
use super::tex::TextureDirectory;
use super::types::{
    ChildId, SectorId, SpecialType, ThingType, WadCoord, WadLinedef, WadName, WadNode, WadSector,
    WadSeg, WadThing,
};
use super::util::{
    from_wad_coords, from_wad_height, is_sky_flat, is_untextured, parse_child_id, to_wad_height,
};
use indexmap::IndexMap;
use log::{debug, error, info, warn};
use math::prelude::*;
use math::{Deg, Line2f, Pnt2f, Pnt3f, Radf, Vec2f};
use std::cmp;
use std::cmp::Ordering;
use std::f32::EPSILON;
use std::mem;
use vec_map::VecMap;

pub struct StaticQuad<'a> {
    pub object_id: ObjectId,
    pub vertices: (Pnt2f, Pnt2f),
    pub tex_start: (f32, f32),
    pub tex_end: (f32, f32),
    pub height_range: (f32, f32),
    pub light_info: &'a LightInfo,
    pub scroll: f32,
    pub tex_name: Option<WadName>,
    pub blocker: bool,
}

pub struct StaticPoly<'a> {
    pub object_id: ObjectId,
    pub vertices: &'a [Pnt2f],
    pub height: f32,
    pub light_info: &'a LightInfo,
    pub tex_name: WadName,
}

pub struct SkyQuad {
    pub object_id: ObjectId,
    pub vertices: (Pnt2f, Pnt2f),
    pub height_range: (f32, f32),
}

pub struct SkyPoly<'a> {
    pub object_id: ObjectId,
    pub vertices: &'a [Pnt2f],
    pub height: f32,
}

pub struct Decor<'a> {
    pub object_id: ObjectId,
    pub low: Pnt3f,
    pub high: Pnt3f,
    pub half_width: f32,
    pub light_info: &'a LightInfo,
    pub tex_name: WadName,
}

pub trait LevelVisitor: Sized {
    fn visit_wall_quad(&mut self, _quad: &StaticQuad) {
        // Default impl is empty to allow visitors to mix and match.
    }

    fn visit_floor_poly(&mut self, _poly: &StaticPoly) {
        // Default impl is empty to allow visitors to mix and match.
    }

    fn visit_ceil_poly(&mut self, _poly: &StaticPoly) {
        // Default impl is empty to allow visitors to mix and match.
    }

    fn visit_floor_sky_poly(&mut self, _poly: &SkyPoly) {
        // Default impl is empty to allow visitors to mix and match.
    }

    fn visit_ceil_sky_poly(&mut self, _poly: &SkyPoly) {
        // Default impl is empty to allow visitors to mix and match.
    }

    fn visit_sky_quad(&mut self, _quad: &SkyQuad) {
        // Default impl is empty to allow visitors to mix and match.
    }

    fn visit_marker(&mut self, _pos: Pnt3f, _yaw: Radf, _marker: Marker) {
        // Default impl is empty to allow visitors to mix and match.
    }

    fn visit_decor(&mut self, _decor: &Decor) {
        // Default impl is empty to allow visitors to mix and match.
    }

    fn visit_bsp_root(&mut self, _line: &Line2f) {
        // Default impl is empty to allow visitors to mix and match.
    }

    fn visit_bsp_node(&mut self, _line: &Line2f, _branch: Branch) {
        // Default impl is empty to allow visitors to mix and match.
    }

    fn visit_bsp_leaf(&mut self, _branch: Branch) {
        // Default impl is empty to allow visitors to mix and match.
    }

    fn visit_bsp_leaf_end(&mut self) {
        // Default impl is empty to allow visitors to mix and match.
    }

    fn visit_bsp_node_end(&mut self) {
        // Default impl is empty to allow visitors to mix and match.
    }

    fn chain<'a, 'b, V: LevelVisitor>(
        &'a mut self,
        other: &'b mut V,
    ) -> VisitorChain<'a, 'b, Self, V> {
        VisitorChain {
            first: self,
            second: other,
        }
    }
}

#[derive(Eq, PartialEq, Debug, Copy, Clone)]
pub enum Branch {
    Positive,
    Negative,
}

#[derive(Eq, PartialEq, Debug, Copy, Clone)]
pub enum Marker {
    StartPos { player: usize },
    TeleportStart,
    TeleportEnd,
}

#[derive(Eq, PartialEq, Debug, Copy, Clone, Default)]
pub struct ObjectId(pub u32);

struct SectorInfo {
    floor_id: ObjectId,
    ceiling_id: ObjectId,
    floor_range: (WadCoord, WadCoord),
    ceiling_range: (WadCoord, WadCoord),
}

impl SectorInfo {
    fn max_height(&self) -> WadCoord {
        self.ceiling_range.1 - self.floor_range.0
    }
}

#[derive(Debug, Default)]
struct DynamicSectorInfo {
    floor_id: ObjectId,
    ceiling_id: ObjectId,
    neighbour_heights: Option<NeighbourHeights>,
    floor_range: Option<(WadCoord, WadCoord)>,
    ceiling_range: Option<(WadCoord, WadCoord)>,
}

impl DynamicSectorInfo {
    fn update(
        &mut self,
        next_dynamic_object_id: &mut ObjectId,
        level: &Level,
        sector_id: SectorId,
        trigger: &mut Trigger,
    ) {
        let sector = &level.sectors[sector_id as usize];
        let effect_def = match trigger.move_effect_def {
            Some(effect_def) => effect_def,
            None => return,
        };

        let heights = if let Some(heights) = self.neighbour_heights {
            heights
        } else if let Some(heights) = level.neighbour_heights(sector) {
            self.neighbour_heights = Some(heights);
            heights
        } else {
            error!(
                "Sector {} has no neighbours, cannot compute its open height.",
                sector_id
            );
            return;
        };

        let (first_floor, second_floor) =
            HeightEffectDef::option_to_heights(effect_def.floor, sector, &heights);
        let (first_ceiling, second_ceiling) =
            HeightEffectDef::option_to_heights(effect_def.ceiling, sector, &heights);
        let repeat = effect_def.repeat;

        merge_range(
            &mut self.floor_range,
            sector.floor_height,
            first_floor.into_iter().chain(second_floor),
        );
        merge_range(
            &mut self.ceiling_range,
            sector.ceiling_height,
            first_ceiling.into_iter().chain(second_ceiling),
        );

        if self.ceiling_range.is_some() && self.ceiling_id == ObjectId(0) {
            self.ceiling_id = *next_dynamic_object_id;
            next_dynamic_object_id.0 += 1;
        }
        if self.floor_range.is_some() && self.floor_id == ObjectId(0) {
            self.floor_id = *next_dynamic_object_id;
            next_dynamic_object_id.0 += 1;
        }

        if let Some(first_floor) = first_floor {
            let offset = from_wad_height(first_floor - sector.floor_height);
            trigger.move_effects.push(MoveEffect {
                object_id: self.floor_id,
                wait: effect_def.wait,
                speed: effect_def.speed,
                first_height_offset: offset,
                second_height_offset: second_floor
                    .map(|floor| from_wad_height(floor - sector.floor_height)),
                repeat,
            });
        }

        if let Some(first_ceiling) = first_ceiling {
            trigger.move_effects.push(MoveEffect {
                object_id: self.ceiling_id,
                wait: effect_def.wait,
                speed: effect_def.speed,
                first_height_offset: from_wad_height(first_ceiling - sector.ceiling_height),
                second_height_offset: second_ceiling
                    .map(|ceiling| from_wad_height(ceiling - sector.ceiling_height)),
                repeat,
            });
        }
    }
}

fn merge_range<I: IntoIterator<Item = WadCoord>>(
    range: &mut Option<(WadCoord, WadCoord)>,
    current: WadCoord,
    with: I,
) {
    *range = with
        .into_iter()
        .fold(*range, |range, coord| {
            Some(match range {
                Some((min, max)) => (min.min(coord), max.max(coord)),
                None => (coord, coord),
            })
        })
        .map(|(min, max)| (min.min(current), max.max(current)));
}

#[derive(Debug, Copy, Clone)]
pub struct MoveEffect {
    pub object_id: ObjectId,
    pub first_height_offset: f32,
    pub second_height_offset: Option<f32>,
    pub speed: f32,
    pub wait: f32,
    pub repeat: bool,
}

impl HeightDef {
    fn to_height(self, sector: &WadSector, heights: &NeighbourHeights) -> Option<WadCoord> {
        let base = match self.to {
            HeightRef::LowestFloor => heights.lowest_floor,
            HeightRef::NextFloor => {
                if let Some(height) = heights.next_floor {
                    height
                } else {
                    return None;
                }
            }
            HeightRef::HighestFloor => heights.highest_floor,
            HeightRef::LowestCeiling => heights.lowest_ceiling,
            HeightRef::HighestCeiling => heights.highest_ceiling,
            HeightRef::Floor => sector.floor_height,
            HeightRef::Ceiling => sector.ceiling_height,
        };
        Some(base + self.offset)
    }
}

impl HeightEffectDef {
    fn option_to_heights(
        this: Option<Self>,
        sector: &WadSector,
        heights: &NeighbourHeights,
    ) -> (Option<WadCoord>, Option<WadCoord>) {
        this.map_or((None, None), |def| {
            (
                def.first.to_height(sector, heights),
                def.second.and_then(|def| def.to_height(sector, heights)),
            )
        })
    }
}

#[derive(Debug, Clone)]
pub struct Trigger {
    pub trigger_type: TriggerType,
    pub line: Line2f,
    pub special_type: SpecialType,
    pub only_once: bool,

    pub unimplemented: bool,
    pub move_effect_def: Option<MoveEffectDef>,
    pub exit_effect: Option<ExitEffectDef>,
    pub move_effects: Vec<MoveEffect>,
}

pub struct LevelAnalysis {
    dynamic_info: IndexMap<SectorId, DynamicSectorInfo>,
    triggers: Vec<Trigger>,
    num_objects: usize,
}

impl LevelAnalysis {
    pub fn new(level: &Level, meta: &WadMetadata) -> Self {
        let mut this = Self {
            dynamic_info: IndexMap::new(),
            triggers: Vec::new(),
            num_objects: 0,
        };
        this.compute_dynamic_sectors(level, meta);
        this
    }

    pub fn num_objects(&self) -> usize {
        self.num_objects
    }

    pub fn take_triggers(&mut self) -> Vec<Trigger> {
        mem::replace(&mut self.triggers, Vec::new())
    }

    fn compute_dynamic_sectors(&mut self, level: &Level, meta: &WadMetadata) {
        info!("Computing dynamic sectors...");
        let mut num_dynamic_linedefs = 0;

        let mut sector_tags_and_ids = level
            .sectors
            .iter()
            .enumerate()
            .filter_map(|(i_sector, sector)| {
                let tag = sector.tag;
                if tag > 0 {
                    Some((tag, i_sector as SectorId))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        sector_tags_and_ids.sort_unstable();

        let max_tag = if let Some(&(max_tag, _)) = sector_tags_and_ids.last() {
            max_tag
        } else {
            return;
        };
        let mut tag_to_first_index = VecMap::with_capacity(max_tag as usize);
        let mut last_tag = !0usize;
        for (i_sector_tag_and_id, &(tag, _)) in sector_tags_and_ids.iter().enumerate() {
            let tag = tag as usize;
            if tag != last_tag {
                tag_to_first_index.insert(tag, i_sector_tag_and_id);
                last_tag = tag;
            }
        }

        let mut next_dynamic_object_id = ObjectId(1);
        for (i_linedef, linedef) in level.linedefs.iter().enumerate() {
            let mut trigger = if let Some(trigger) = self.linedef_to_trigger(level, meta, linedef) {
                trigger
            } else {
                continue;
            };
            num_dynamic_linedefs += 1;

            let tag = linedef.sector_tag;
            if tag == 0 {
                if let Some(sidedef) = level.left_sidedef(linedef) {
                    let left_sector_id = sidedef.sector;
                    debug!(
                        "Sector {} with zero tag marked as dynamic, required by manual linedef {}.",
                        left_sector_id, i_linedef
                    );
                    self.dynamic_info
                        .entry(left_sector_id)
                        .or_insert_with(DynamicSectorInfo::default)
                        .update(
                            &mut next_dynamic_object_id,
                            level,
                            left_sector_id,
                            &mut trigger,
                        );
                }
                self.triggers.push(trigger);
                continue;
            }

            if let Some(first_index) = tag_to_first_index.get(tag as usize) {
                for &(current_tag, current_sector_id) in &sector_tags_and_ids[*first_index..] {
                    if current_tag != tag {
                        break;
                    }
                    debug!(
                        "Sector {} with the tag {} marked as dynamic, required by linedef {}.",
                        current_sector_id, tag, i_linedef
                    );
                    self.dynamic_info
                        .entry(current_sector_id)
                        .or_insert_with(DynamicSectorInfo::default)
                        .update(
                            &mut next_dynamic_object_id,
                            level,
                            current_sector_id,
                            &mut trigger,
                        );
                }
            } else {
                warn!(
                    "No sector with the tag {}, required by linedef {}.",
                    tag, i_linedef
                );
            }
            self.triggers.push(trigger);
        }
        for (i_trigger, trigger) in self.triggers.iter().enumerate() {
            debug!("Trigger {}: {:#?}", i_trigger, trigger);
        }
        for (i_object, dynamic) in &self.dynamic_info {
            debug!("Dynamic {}: {:#?}", i_object, dynamic);
        }
        self.num_objects = next_dynamic_object_id.0 as usize;
        info!(
            "Finished computing dynamic sectors: num_dynamic_sectors={} num_dynamic_linedefs={}",
            self.num_objects, num_dynamic_linedefs
        );
    }

    fn linedef_to_trigger(
        &self,
        level: &Level,
        meta: &WadMetadata,
        linedef: &WadLinedef,
    ) -> Option<Trigger> {
        let special_type = linedef.special_type;
        if special_type == 0 {
            return None;
        }

        let line = match (
            level.vertex(linedef.start_vertex),
            level.vertex(linedef.end_vertex),
        ) {
            (Some(start), Some(end)) => Line2f::from_two_points(start, end),
            _ => {
                error!("Missing vertices for linedef, skipping.");
                return None;
            }
        };

        Some(if let Some(meta) = meta.linedef.get(&special_type) {
            Trigger {
                trigger_type: meta.trigger,

                only_once: meta.only_once,
                move_effect_def: meta.move_effect,
                exit_effect: meta.exit_effect,
                unimplemented: false,
                special_type,

                line,
                move_effects: Vec::new(),
            }
        } else {
            error!("Unknown linedef special type: {}", special_type);
            Trigger {
                trigger_type: TriggerType::Any,

                only_once: false,
                move_effect_def: None,
                exit_effect: None,
                unimplemented: true,
                special_type,

                line,
                move_effects: Vec::new(),
            }
        })
    }
}

pub struct LevelWalker<'a, V: LevelVisitor + 'a> {
    level: &'a Level,
    tex: &'a TextureDirectory,
    meta: &'a WadMetadata,
    visitor: &'a mut V,
    height_range: (WadCoord, WadCoord),
    bsp_lines: Vec<Line2f>,

    dynamic_info: &'a IndexMap<SectorId, DynamicSectorInfo>,

    // The vector contains all (2D) points which are part of the subsector:
    // implicit (intersection of BSP lines) and explicit (seg vertices).
    subsector_points: Vec<Pnt2f>,
    subsector_seg_lines: Vec<Line2f>,

    // A cache of computed LightInfo per sector, to avoid recalculating.
    light_cache: VecMap<LightInfo>,
}

impl<'a, V: LevelVisitor> LevelWalker<'a, V> {
    pub fn new(
        level: &'a Level,
        analysis: &'a LevelAnalysis,
        tex: &'a TextureDirectory,
        meta: &'a WadMetadata,
        visitor: &'a mut V,
    ) -> Self {
        Self {
            level,
            tex,
            meta,
            visitor,
            height_range: min_max_height(level),
            bsp_lines: Vec::with_capacity(32),
            subsector_points: Vec::with_capacity(32),
            subsector_seg_lines: Vec::with_capacity(32),
            light_cache: VecMap::with_capacity(level.sectors.len()),

            dynamic_info: &analysis.dynamic_info,
        }
    }

    pub fn walk(&mut self) {
        let root = match self.level.nodes.last() {
            Some(node) => node,
            None => {
                warn!("Level contains no nodes, visitor not called at all.");
                return;
            }
        };
        let partition = partition_line(root);
        self.visitor.visit_bsp_root(&partition);
        self.children(root, partition);
        self.visitor.visit_bsp_node_end();

        self.things();
    }

    fn floor_id(&self, sector: &WadSector) -> ObjectId {
        self.dynamic_info
            .get(&self.level.sector_id(sector))
            .map_or(ObjectId(0), |dynamic| dynamic.floor_id)
    }

    fn ceiling_id(&self, sector: &WadSector) -> ObjectId {
        self.dynamic_info
            .get(&self.level.sector_id(sector))
            .map_or(ObjectId(0), |dynamic| dynamic.ceiling_id)
    }

    fn sector_info(&self, sector: &WadSector) -> SectorInfo {
        let floor_range = (sector.floor_height, sector.floor_height);
        let ceiling_range = (sector.ceiling_height, sector.ceiling_height);
        self.dynamic_info
            .get(&self.level.sector_id(sector))
            .map_or_else(
                || SectorInfo {
                    floor_id: ObjectId(0),
                    ceiling_id: ObjectId(0),
                    floor_range,
                    ceiling_range,
                },
                |dynamic_info| SectorInfo {
                    floor_id: dynamic_info.floor_id,
                    ceiling_id: dynamic_info.ceiling_id,
                    floor_range: dynamic_info.floor_range.unwrap_or(floor_range),
                    ceiling_range: dynamic_info.ceiling_range.unwrap_or(ceiling_range),
                },
            )
    }

    fn node(&mut self, id: ChildId, branch: Branch) {
        let (id, is_leaf) = parse_child_id(id);
        if is_leaf {
            self.visitor.visit_bsp_leaf(branch);
            self.subsector(id);
            self.visitor.visit_bsp_leaf_end();
            return;
        }

        let node = if let Some(node) = self.level.nodes.get(id) {
            node
        } else {
            warn!("Missing entire node with id {}, skipping.", id);
            return;
        };
        let partition = partition_line(node);
        self.visitor.visit_bsp_node(&partition, branch);
        self.children(node, partition);
        self.visitor.visit_bsp_node_end();
    }

    fn children(&mut self, node: &WadNode, partition: Line2f) {
        self.bsp_lines.push(partition);
        self.node(node.left, Branch::Positive);
        self.bsp_lines.pop();

        self.bsp_lines.push(partition.inverted_halfspaces());
        self.node(node.right, Branch::Negative);
        self.bsp_lines.pop();
    }

    fn subsector(&mut self, id: usize) {
        let subsector = if let Some(subsector) = self.level.ssector(id) {
            subsector
        } else {
            warn!("Cannot find subsector with id {}, will skip.", id);
            return;
        };
        let segs = if let Some(segs) = self.level.ssector_segs(subsector) {
            segs
        } else {
            warn!("Cannot find segs for subsector with id {}, will skip.", id);
            return;
        };
        if segs.is_empty() {
            warn!("Zero segs for subsector with id {}, will skip.", id);
            return;
        }
        let sector = if let Some(sector) = self.level.seg_sector(&segs[0]) {
            sector
        } else {
            warn!("Cannot find subsector with id {}, will skip.", id);
            return;
        };
        let sector_info = self.sector_info(sector);

        // These vectors get cleared for every subsector, we're just reusing the
        // allocations.
        self.subsector_seg_lines.clear();
        self.subsector_seg_lines.reserve(segs.len());
        self.subsector_points.clear();
        self.subsector_points.reserve(segs.len() * 3);

        // First add the explicit points.
        for seg in segs {
            let (v1, v2) = if let Some(vertices) = self.level.seg_vertices(seg) {
                vertices
            } else {
                warn!("Cannot find seg vertices in subsector {}, will skip.", id);
                return;
            };
            self.subsector_points.push(v1);
            self.subsector_points.push(v2);
            self.subsector_seg_lines
                .push(Line2f::from_two_points(v1, v2));

            // Also push the wall segments.
            self.seg(sector, &sector_info, seg, (v1, v2));
        }

        // The convex polyon defined at the intersection of the partition lines,
        // intersected with the half-volumes of the segs form the 'implicit' points.
        for i_line in 0..(self.bsp_lines.len() - 1) {
            for j_line in (i_line + 1)..self.bsp_lines.len() {
                let (l1, l2) = (&self.bsp_lines[i_line], &self.bsp_lines[j_line]);
                let point = match l1.intersect_point(l2) {
                    Some(p) => p,
                    None => continue,
                };

                let dist = |l: &Line2f| l.signed_distance(point);
                let within_bsp = |d: f32| d >= -BSP_TOLERANCE;
                let within_seg = |d: f32| d <= SEG_TOLERANCE;
                // The intersection point must lie both within the BSP volume
                // and the segs volume.
                let inside_bsp_and_segs = self.bsp_lines.iter().map(&dist).all(within_bsp)
                    && self.subsector_seg_lines.iter().map(&dist).all(within_seg);
                if inside_bsp_and_segs {
                    self.subsector_points.push(point);
                }
            }
        }
        if self.subsector_points.len() < 3 {
            warn!(
                "Degenerate source polygon {} ({} vertices).",
                id,
                self.subsector_points.len()
            );
        }
        points_to_polygon(&mut self.subsector_points); // Sort and remove duplicates.
        if self.subsector_points.len() < 3 {
            warn!(
                "Degenerate cannonicalised polygon {} ({} vertices).",
                id,
                self.subsector_points.len()
            );
        } else {
            self.flat_poly(sector, &sector_info);
        }
    }

    fn seg(
        &mut self,
        sector: &WadSector,
        info: &SectorInfo,
        seg: &WadSeg,
        vertices: (Pnt2f, Pnt2f),
    ) {
        let line = if let Some(line) = self.level.seg_linedef(seg) {
            line
        } else {
            warn!("No linedef found for seg, skipping seg.");
            return;
        };
        let sidedef = if let Some(sidedef) = self.level.seg_sidedef(seg) {
            sidedef
        } else {
            warn!("No sidedef found for seg, skipping seg.");
            return;
        };
        let (min, max) = (self.height_range.0, self.height_range.1);
        let (floor, ceiling) = (sector.floor_height, sector.ceiling_height);
        let unpeg_lower = line.lower_unpegged();
        let back_sector = match self.level.seg_back_sector(seg) {
            None => {
                self.wall_quad(InternalWallQuad {
                    sector,
                    seg,
                    vertices,
                    object_id: if unpeg_lower {
                        info.floor_id
                    } else {
                        info.ceiling_id
                    },
                    height_range: if unpeg_lower {
                        (floor, floor + info.max_height())
                    } else {
                        (ceiling - info.max_height(), ceiling)
                    },
                    texture_name: sidedef.middle_texture,
                    peg: if unpeg_lower { Peg::Bottom } else { Peg::Top },
                    blocker: true,
                });
                if is_sky_flat(sector.ceiling_texture) {
                    self.sky_quad(info.ceiling_id, vertices, (ceiling, max));
                }
                if is_sky_flat(sector.floor_texture) {
                    self.sky_quad(info.floor_id, vertices, (min, floor));
                }
                return;
            }
            Some(sector) => sector,
        };
        let (back_floor, back_ceiling) = (back_sector.floor_height, back_sector.ceiling_height);
        let back_info = self.sector_info(back_sector);

        if is_sky_flat(sector.ceiling_texture) && !is_sky_flat(back_sector.ceiling_texture) {
            self.sky_quad(info.ceiling_id, vertices, (ceiling, max));
        }
        if is_sky_flat(sector.floor_texture) && !is_sky_flat(back_sector.floor_texture) {
            self.sky_quad(info.floor_id, vertices, (min, floor));
        }

        let unpeg_upper = line.upper_unpegged();
        let floor = if back_info.floor_range.1 > info.floor_range.0 {
            self.wall_quad(InternalWallQuad {
                sector,
                seg,
                vertices,
                object_id: back_info.floor_id,
                height_range: (
                    back_floor - back_info.floor_range.1 + info.floor_range.0,
                    back_floor,
                ),
                texture_name: sidedef.lower_texture,
                peg: if unpeg_lower {
                    Peg::BottomLower
                } else {
                    Peg::Top
                },
                blocker: true,
            });
            back_floor
        } else {
            floor
        };
        let ceil = if back_ceiling < ceiling {
            if !is_sky_flat(back_sector.ceiling_texture) {
                self.wall_quad(InternalWallQuad {
                    sector,
                    seg,
                    vertices,
                    object_id: back_info.ceiling_id,
                    height_range: (back_ceiling, ceiling),
                    texture_name: sidedef.upper_texture,
                    peg: if unpeg_upper { Peg::Top } else { Peg::Bottom },
                    blocker: true,
                });
            }
            back_ceiling
        } else {
            ceiling
        };
        self.wall_quad(InternalWallQuad {
            sector,
            seg,
            vertices,
            object_id: if unpeg_lower {
                info.floor_id
            } else {
                info.ceiling_id
            },
            height_range: (floor, ceil),
            texture_name: sidedef.middle_texture,
            peg: if unpeg_lower {
                if is_untextured(sidedef.upper_texture) {
                    Peg::TopFloat
                } else {
                    Peg::Bottom
                }
            } else if is_untextured(sidedef.lower_texture) {
                Peg::BottomFloat
            } else {
                Peg::Top
            },
            blocker: line.impassable(),
        });
    }

    fn wall_quad(&mut self, quad: InternalWallQuad) {
        let InternalWallQuad {
            object_id,
            sector,
            seg,
            vertices: (v1, v2),
            height_range: (low, high),
            texture_name,
            peg,
            blocker,
        } = quad;
        if low >= high {
            return;
        }
        let size = if is_untextured(texture_name) {
            None
        } else if let Some(image) = self.tex.texture(texture_name) {
            Some(Pnt2f::new(image.width() as f32, image.height() as f32))
        } else {
            warn!("wall_quad: No such wall texture '{}'", texture_name);
            return;
        };
        let line = if let Some(line) = self.level.seg_linedef(seg) {
            line
        } else {
            warn!("Missing linedef for seg, skipping wall.");
            return;
        };
        let sidedef = if let Some(sidedef) = self.level.seg_sidedef(seg) {
            sidedef
        } else {
            warn!("Missing sidedef for seg, skipping wall.");
            return;
        };
        let bias = (v2 - v1).normalize_or_zero() * POLY_BIAS;
        let (v1, v2) = (v1 + (-bias), v2 + bias);
        let (low, high) = match (size, peg) {
            (Some(size), Peg::TopFloat) => (
                from_wad_height(low + sidedef.y_offset),
                from_wad_height(low + size[1] as i16 + sidedef.y_offset),
            ),
            (Some(size), Peg::BottomFloat) => (
                from_wad_height(high + sidedef.y_offset - size[1] as i16),
                from_wad_height(high + sidedef.y_offset),
            ),
            _ => (from_wad_height(low), from_wad_height(high)),
        };

        let light_info_with_contrast;
        let light_info = light_info(&mut self.light_cache, self.level, sector);
        let light_info = if light_info.effect.is_none() {
            if (v1[0] - v2[0]).abs() < EPSILON {
                light_info_with_contrast = light::with_contrast(light_info, Contrast::Brighten);
                &light_info_with_contrast
            } else if (v1[1] - v2[1]).abs() < EPSILON {
                light_info_with_contrast = light::with_contrast(light_info, Contrast::Darken);
                &light_info_with_contrast
            } else {
                light_info
            }
        } else {
            light_info
        };

        let height = to_wad_height(high - low);
        let s1 = f32::from(seg.offset) + f32::from(sidedef.x_offset);
        let s2 = s1 + to_wad_height((v2 - v1).magnitude());
        let (t1, t2) = match (size, peg) {
            (Some(_), Peg::Top) | (None, _) => (height, 0.0),
            (Some(size), Peg::Bottom) => (size[1], size[1] - height),
            (Some(size), Peg::BottomLower) => {
                // As far as I can tell, this is a special case.
                let sector_height = f32::from(sector.ceiling_height - sector.floor_height);
                (size[1] + sector_height, size[1] - height + sector_height)
            }
            (Some(size), Peg::TopFloat) | (Some(size), Peg::BottomFloat) => (size[1], 0.0),
        };
        let (t1, t2) = (
            t1 + f32::from(sidedef.y_offset),
            t2 + f32::from(sidedef.y_offset),
        );

        // TODO(cristicbz): Magic numbers below.
        let scroll = if line.special_type == 0x30 { 35.0 } else { 0.0 };

        let (low, high) = (low - POLY_BIAS, high + POLY_BIAS);

        self.visitor.visit_wall_quad(&StaticQuad {
            vertices: (v1, v2),
            tex_start: (s1, t1),
            tex_end: (s2, t2),
            height_range: (low, high),
            light_info,
            tex_name: size.map(|_| texture_name),
            blocker,
            scroll,
            object_id,
        });
    }

    fn flat_poly(&mut self, sector: &WadSector, info: &SectorInfo) {
        let light_info = light_info(&mut self.light_cache, self.level, sector);
        let (floor_tex, ceil_tex) = (sector.floor_texture, sector.ceiling_texture);
        let (floor_sky, ceil_sky) = (is_sky_flat(floor_tex), is_sky_flat(ceil_tex));
        let floor_y = from_wad_height(if floor_sky {
            self.height_range.0
        } else {
            sector.floor_height
        });
        let ceil_y = from_wad_height(if ceil_sky {
            self.height_range.1
        } else {
            sector.ceiling_height
        });

        if floor_sky {
            self.visitor.visit_floor_sky_poly(&SkyPoly {
                object_id: info.floor_id,
                vertices: &self.subsector_points,
                height: floor_y,
            });
        } else {
            self.visitor.visit_floor_poly(&StaticPoly {
                object_id: info.floor_id,
                vertices: &self.subsector_points,
                height: floor_y,
                light_info,
                tex_name: floor_tex,
            });
        }

        if ceil_sky {
            self.visitor.visit_ceil_sky_poly(&SkyPoly {
                object_id: info.ceiling_id,
                vertices: &self.subsector_points,
                height: ceil_y,
            });
        } else {
            self.visitor.visit_ceil_poly(&StaticPoly {
                object_id: info.ceiling_id,
                vertices: &self.subsector_points,
                height: ceil_y,
                light_info,
                tex_name: ceil_tex,
            });
        }
    }

    fn sky_quad(
        &mut self,
        object_id: ObjectId,
        (v1, v2): (Pnt2f, Pnt2f),
        (low, high): (WadCoord, WadCoord),
    ) {
        if low >= high {
            return;
        }
        let edge = (v2 - v1).normalize_or_zero();
        let bias = edge * POLY_BIAS * 16.0;
        let normal = Vec2f::new(-edge[1], edge[0]);
        let normal_bias = normal * POLY_BIAS * 16.0;
        let (v1, v2) = (v1 + (normal_bias - bias), v2 + (normal_bias + bias));
        let (low, high) = (from_wad_height(low), from_wad_height(high));

        self.visitor.visit_sky_quad(&SkyQuad {
            object_id,
            vertices: (v1, v2),
            height_range: (low, high),
        });
    }

    fn things(&mut self) {
        for thing in &self.level.things {
            let pos = from_wad_coords(thing.x, thing.y);
            let yaw = Deg(f32::round(f32::from(thing.angle) / 45.0) * 45.0);
            let sector = match self.sector_at(pos) {
                Some(sector) => sector,
                None => continue,
            };

            if let Some(marker) = Marker::from(thing.thing_type) {
                let pos = Pnt3f::new(pos[0], from_wad_height(sector.floor_height), pos[1]);
                self.visitor.visit_marker(pos, yaw.into(), marker);
            } else if let Some(sector) = self.sector_at(pos) {
                self.decor(thing, pos, sector);
            }
        }
    }

    fn sector_at(&self, pos: Pnt2f) -> Option<&'a WadSector> {
        let mut child_id = (self.level.nodes.len() - 1) as ChildId;
        loop {
            let (id, is_leaf) = parse_child_id(child_id);
            if is_leaf {
                let segs = self
                    .level
                    .ssector(id)
                    .and_then(|subsector| self.level.ssector_segs(subsector))
                    .and_then(|segs| if segs.is_empty() { None } else { Some(segs) });
                let segs = if let Some(segs) = segs {
                    segs
                } else {
                    return None;
                };
                let sector = if let Some(sector) = self.level.seg_sector(&segs[0]) {
                    sector
                } else {
                    return None;
                };
                return if segs
                    .iter()
                    .filter_map(|seg| self.level.seg_vertices(seg))
                    .map(|(v1, v2)| Line2f::from_two_points(v1, v2))
                    .all(|line| line.signed_distance(pos) <= SEG_TOLERANCE)
                {
                    Some(sector)
                } else {
                    None
                };
            } else {
                let node = if let Some(node) = self.level.nodes.get(id) {
                    node
                } else {
                    return None;
                };
                let partition = partition_line(node);
                if partition.signed_distance(pos) > 0.0f32 {
                    child_id = node.left;
                } else {
                    child_id = node.right;
                }
            }
        }
    }

    fn decor(&mut self, thing: &WadThing, pos: Pnt2f, sector: &WadSector) {
        let meta = match self.meta.find_thing(thing.thing_type) {
            Some(m) => m,
            None => {
                warn!("No metadata found for thing type {}", thing.thing_type);
                return;
            }
        };
        let (name, size) = {
            let mut sprite0 = meta.sprite;
            // Ignore the error: if this fails, so will the `sprite0` and `sprite1` pushes below.
            let _ = sprite0.push(meta.sequence.as_bytes()[0]);
            let mut sprite1 = sprite0;
            let sprite0 = sprite0.push(b'0').ok().map(|_| sprite0);
            let sprite1 = sprite1.push(b'1').ok().map(|_| sprite1);

            match (sprite0, sprite1) {
                (Some(sprite0), Some(sprite1)) => {
                    if let Some(image) = self.tex.texture(sprite0) {
                        (sprite0, image.size())
                    } else if let Some(image) = self.tex.texture(sprite1) {
                        (sprite1, image.size())
                    } else {
                        warn!(
                            "No such sprite {} for thing {}",
                            meta.sprite, thing.thing_type
                        );
                        return;
                    }
                }
                _ => {
                    warn!(
                        "Metadata sprite name ({}) for thing type {} is not a valid WadName.",
                        meta.sprite, thing.thing_type
                    );
                    return;
                }
            }
        };
        let size = Vec2f::new(
            from_wad_height(size[0] as i16),
            from_wad_height(size[1] as i16),
        );

        let (object_id, low, high) = if meta.hanging {
            (
                self.ceiling_id(sector),
                Pnt3f::new(
                    pos[0],
                    from_wad_height(sector.ceiling_height) - size[1],
                    pos[1],
                ),
                Pnt3f::new(pos[0], from_wad_height(sector.ceiling_height), pos[1]),
            )
        } else {
            (
                self.floor_id(sector),
                Pnt3f::new(pos[0], from_wad_height(sector.floor_height), pos[1]),
                Pnt3f::new(
                    pos[0],
                    from_wad_height(sector.floor_height) + size[1],
                    pos[1],
                ),
            )
        };
        let half_width = size[0] * 0.5;

        self.visitor.visit_decor(&Decor {
            object_id,
            low,
            high,
            half_width,
            light_info: light_info(&mut self.light_cache, self.level, sector),
            tex_name: name,
        });
    }
}

fn light_info<'a>(
    cache: &'a mut VecMap<LightInfo>,
    level: &Level,
    sector: &WadSector,
) -> &'a LightInfo {
    cache
        .entry(level.sector_id(sector) as usize)
        .or_insert_with(|| light::new_light(level, sector))
}

fn partition_line(node: &WadNode) -> Line2f {
    Line2f::from_two_points(
        from_wad_coords(node.line_x, node.line_y),
        from_wad_coords(node.line_x + node.step_x, node.line_y + node.step_y),
    )
}

// Distance on the wrong side of a BSP and seg line allowed.
const BSP_TOLERANCE: f32 = 1e-3;
const SEG_TOLERANCE: f32 = 0.1;

// All polygons are `fattened' by this amount to fill in thin gaps between them.
const POLY_BIAS: f32 = 0.64 * 3e-4;

#[derive(Copy, Clone)]
enum Peg {
    Top,
    Bottom,
    BottomLower,
    TopFloat,
    BottomFloat,
}

fn min_max_height(level: &Level) -> (WadCoord, WadCoord) {
    let (min, max) = level
        .sectors
        .iter()
        .map(|s| (s.floor_height, s.ceiling_height))
        .fold((32_767, -32_768), |(min, max), (f, c)| {
            (cmp::min(min, f), cmp::max(max, c))
        });
    (min - 512, max + 512)
}

fn polygon_center(points: &[Pnt2f]) -> Pnt2f {
    let mut center = Pnt2f::origin();
    for p in points.iter() {
        center += p.to_vec();
    }
    center / (points.len() as f32)
}

fn points_to_polygon(points: &mut Vec<Pnt2f>) {
    // Sort points in polygonal CCW order around their center.
    let center = polygon_center(points);
    points.sort_unstable_by(|a, b| {
        let ac = *a - center;
        let bc = *b - center;
        if ac[0] >= 0.0 && bc[0] < 0.0 {
            return Ordering::Less;
        }
        if ac[0] < 0.0 && bc[0] >= 0.0 {
            return Ordering::Greater;
        }
        if ac[0] == 0.0 && bc[0] == 0.0 {
            if ac[1] >= 0.0 || bc[1] >= 0.0 {
                return if a[1] > b[1] {
                    Ordering::Less
                } else {
                    Ordering::Greater
                };
            }
            return if b[1] > a[1] {
                Ordering::Less
            } else {
                Ordering::Greater
            };
        }

        if ac.perp_dot(bc) < 0.0 {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    });

    // Remove duplicates.
    let mut simplified = Vec::new();
    simplified.push((*points)[0]);
    let mut current_point = (*points)[1];
    let mut area = 0.0;
    for i_point in 2..points.len() {
        let next_point = (*points)[i_point];
        let prev_point = simplified[simplified.len() - 1];
        let new_area = (next_point - current_point).perp_dot(current_point - prev_point) * 0.5;
        if new_area >= 0.0 {
            if area + new_area > 1.024e-5 {
                area = 0.0;
                simplified.push(current_point);
            } else {
                area += new_area;
            }
        }
        current_point = next_point;
    }
    simplified.push((*points)[points.len() - 1]);
    if simplified.len() < 3 {
        points.clear();
        return;
    }
    while (simplified[0] - simplified[simplified.len() - 1]).magnitude() < 0.0032 {
        simplified.pop();
    }

    let center = polygon_center(&simplified);
    for point in &mut simplified {
        *point += (*point - center).normalize_or_zero() * POLY_BIAS;
    }
    *points = simplified;
}

pub struct VisitorChain<'a, 'b, A: LevelVisitor + 'a, B: LevelVisitor + 'b> {
    first: &'a mut A,
    second: &'b mut B,
}

impl<'a, 'b, A: LevelVisitor, B: LevelVisitor> LevelVisitor for VisitorChain<'a, 'b, A, B> {
    fn visit_wall_quad(&mut self, quad: &StaticQuad) {
        self.first.visit_wall_quad(quad);
        self.second.visit_wall_quad(quad);
    }

    fn visit_floor_poly(&mut self, poly: &StaticPoly) {
        self.first.visit_floor_poly(poly);
        self.second.visit_floor_poly(poly);
    }

    fn visit_ceil_poly(&mut self, poly: &StaticPoly) {
        self.first.visit_ceil_poly(poly);
        self.second.visit_ceil_poly(poly);
    }

    fn visit_floor_sky_poly(&mut self, poly: &SkyPoly) {
        self.first.visit_floor_sky_poly(poly);
        self.second.visit_floor_sky_poly(poly);
    }

    fn visit_ceil_sky_poly(&mut self, poly: &SkyPoly) {
        self.first.visit_ceil_sky_poly(poly);
        self.second.visit_ceil_sky_poly(poly);
    }

    fn visit_sky_quad(&mut self, quad: &SkyQuad) {
        self.first.visit_sky_quad(quad);
        self.second.visit_sky_quad(quad);
    }

    fn visit_marker(&mut self, pos: Pnt3f, yaw: Radf, marker: Marker) {
        self.first.visit_marker(pos, yaw, marker);
        self.second.visit_marker(pos, yaw, marker);
    }

    fn visit_decor(&mut self, decor: &Decor) {
        self.first.visit_decor(decor);
        self.second.visit_decor(decor);
    }

    fn visit_bsp_root(&mut self, line: &Line2f) {
        self.first.visit_bsp_root(line);
        self.second.visit_bsp_root(line);
    }

    fn visit_bsp_node(&mut self, line: &Line2f, branch: Branch) {
        self.first.visit_bsp_node(line, branch);
        self.second.visit_bsp_node(line, branch);
    }

    fn visit_bsp_leaf(&mut self, branch: Branch) {
        self.first.visit_bsp_leaf(branch);
        self.second.visit_bsp_leaf(branch);
    }

    fn visit_bsp_leaf_end(&mut self) {
        self.first.visit_bsp_leaf_end();
        self.second.visit_bsp_leaf_end();
    }

    fn visit_bsp_node_end(&mut self) {
        self.first.visit_bsp_node_end();
        self.second.visit_bsp_node_end();
    }
}

#[derive(Copy, Clone)]
struct InternalWallQuad<'a> {
    object_id: ObjectId,
    sector: &'a WadSector,
    seg: &'a WadSeg,
    vertices: (Pnt2f, Pnt2f),
    height_range: (WadCoord, WadCoord),
    texture_name: WadName,
    peg: Peg,
    blocker: bool,
}

const THING_TYPE_PLAYER1_START: ThingType = 1;
const THING_TYPE_PLAYER2_START: ThingType = 2;
const THING_TYPE_PLAYER3_START: ThingType = 3;
const THING_TYPE_PLAYER4_START: ThingType = 4;
const THING_TYPE_TELEPORT_START: ThingType = 11;
const THING_TYPE_TELEPORT_END: ThingType = 14;

impl Marker {
    fn from(thing_type: ThingType) -> Option<Self> {
        match thing_type {
            THING_TYPE_PLAYER1_START => Some(Marker::StartPos { player: 0 }),
            THING_TYPE_PLAYER2_START => Some(Marker::StartPos { player: 1 }),
            THING_TYPE_PLAYER3_START => Some(Marker::StartPos { player: 2 }),
            THING_TYPE_PLAYER4_START => Some(Marker::StartPos { player: 3 }),
            THING_TYPE_TELEPORT_START => Some(Marker::TeleportStart),
            THING_TYPE_TELEPORT_END => Some(Marker::TeleportEnd),
            _ => None,
        }
    }
}
