use level::Level;
use light::{self, Contrast, LightInfo};
use meta::WadMetadata;
use math::{Line2f, Vec2f, Vec3f, Vector};
use num::Zero;
use std::cmp;
use std::cmp::Ordering;
use tex::TextureDirectory;
use types::{ChildId, WadCoord, WadName, WadSector, WadSeg, WadThing, WadNode, ThingType};
use util::{from_wad_coords, from_wad_height, is_sky_flat, is_untextured, parse_child_id};
use vec_map::VecMap;

pub trait LevelVisitor: Sized {
    fn visit_wall_quad(&mut self,
                       _vertices: &(Vec2f, Vec2f),
                       _tex_start: (f32, f32),
                       _tex_end: (f32, f32),
                       _height_range: (f32, f32),
                       _light_info: &LightInfo,
                       _scroll: f32,
                       _tex_name: Option<&WadName>,
                       _blocker: bool) {
        // Default impl is empty to allow visitors to mix and match.
    }

    fn visit_floor_poly(&mut self,
                        _points: &[Vec2f],
                        _height: f32,
                        _light_info: &LightInfo,
                        _tex_name: &WadName) {
        // Default impl is empty to allow visitors to mix and match.
    }

    fn visit_ceil_poly(&mut self,
                       _points: &[Vec2f],
                       _height: f32,
                       _light_info: &LightInfo,
                       _tex_name: &WadName) {
        // Default impl is empty to allow visitors to mix and match.
    }

    fn visit_floor_sky_poly(&mut self, _points: &[Vec2f], _height: f32) {
        // Default impl is empty to allow visitors to mix and match.
    }

    fn visit_ceil_sky_poly(&mut self, _points: &[Vec2f], _height: f32) {
        // Default impl is empty to allow visitors to mix and match.
    }

    fn visit_sky_quad(&mut self, _vertices: &(Vec2f, Vec2f), _height_range: (f32, f32)) {
        // Default impl is empty to allow visitors to mix and match.
    }

    fn visit_marker(&mut self, _pos: Vec3f, _marker: Marker) {
        // Default impl is empty to allow visitors to mix and match.
    }

    fn visit_decor(&mut self,
                   _low: &Vec3f,
                   _high: &Vec3f,
                   _half_width: f32,
                   _light_info: &LightInfo,
                   _tex_name: &WadName) {
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

    fn chain<'a, 'b, V: LevelVisitor>(&'a mut self,
                                      other: &'b mut V)
                                      -> VisitorChain<'a, 'b, Self, V> {
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
    StartPos {
        player: usize,
    },
    TeleportStart,
    TeleportEnd,
}


pub struct LevelWalker<'a, V: LevelVisitor + 'a> {
    level: &'a Level,
    tex: &'a TextureDirectory,
    meta: &'a WadMetadata,
    visitor: &'a mut V,
    height_range: (WadCoord, WadCoord),
    bsp_lines: Vec<Line2f>,

    // The vector contains all (2D) points which are part of the subsector:
    // implicit (intersection of BSP lines) and explicit (seg vertices).
    subsector_points: Vec<Vec2f>,
    subsector_seg_lines: Vec<Line2f>,

    // A cache of computed LightInfo per sector, to avoid recalculating.
    light_cache: VecMap<LightInfo>,
}

impl<'a, V: LevelVisitor> LevelWalker<'a, V> {
    pub fn new(level: &'a Level,
               tex: &'a TextureDirectory,
               meta: &'a WadMetadata,
               visitor: &'a mut V)
               -> LevelWalker<'a, V> {
        LevelWalker {
            level: level,
            tex: tex,
            meta: meta,
            visitor: visitor,
            height_range: min_max_height(level),
            bsp_lines: Vec::with_capacity(32),
            subsector_points: Vec::with_capacity(32),
            subsector_seg_lines: Vec::with_capacity(32),
            light_cache: VecMap::with_capacity(level.sectors.len()),
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
        let partition = partition_line(&root);
        self.visitor.visit_bsp_root(&partition);
        self.children(root, partition);
        self.visitor.visit_bsp_node_end();

        self.things();
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
        let partition = partition_line(&node);
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
            self.subsector_seg_lines.push(Line2f::from_two_points(v1, v2));

            // Also push the wall segments.
            self.seg(sector, seg, (v1, v2));
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

                let dist = |l: &Line2f| l.signed_distance(&point);

                // The intersection point must lie both within the BSP volume
                // and the segs volume.
                if self.bsp_lines.iter().map(|x| dist(x)).all(|d| d >= -BSP_TOLERANCE) &&
                   self.subsector_seg_lines.iter().map(dist).all(|d| d <= SEG_TOLERANCE) {
                    self.subsector_points.push(point);
                }
            }
        }
        if self.subsector_points.len() < 3 {
            warn!("Degenerate source polygon {} ({} vertices).",
                  id,
                  self.subsector_points.len());
        }
        points_to_polygon(&mut self.subsector_points);  // Sort and remove duplicates.
        if self.subsector_points.len() < 3 {
            warn!("Degenerate cannonicalised polygon {} ({} vertices).",
                  id,
                  self.subsector_points.len());
        } else {
            self.flat_poly(sector);
        }
    }

    fn seg(&mut self, sector: &WadSector, seg: &WadSeg, vertices: (Vec2f, Vec2f)) {
        let line = if let Some(line) = self.level.seg_linedef(seg) {
            line
        } else {
            warn!("No linedef found for seg, skipping seg.");
            return;
        };
        let side = if let Some(side) = self.level.seg_sidedef(seg) {
            side
        } else {
            warn!("No sidedef found for seg, skipping seg.");
            return;
        };
        let (min, max) = (self.height_range.0, self.height_range.1);
        let (floor, ceil) = (sector.floor_height, sector.ceiling_height);
        let unpeg_lower = line.lower_unpegged();
        let back_sector = match self.level.seg_back_sector(seg) {
            None => {
                self.wall_quad(sector,
                               seg,
                               vertices,
                               (floor, ceil),
                               &side.middle_texture,
                               if unpeg_lower {
                                   Peg::Bottom
                               } else {
                                   Peg::Top
                               },
                               true);
                if is_sky_flat(&sector.ceiling_texture) {
                    self.sky_quad(vertices, (ceil, max));
                }
                if is_sky_flat(&sector.floor_texture) {
                    self.sky_quad(vertices, (min, floor));
                }
                return;
            }
            Some(s) => s,
        };

        if is_sky_flat(&sector.ceiling_texture) && !is_sky_flat(&back_sector.ceiling_texture) {
            self.sky_quad(vertices, (ceil, max));
        }
        if is_sky_flat(&sector.floor_texture) && !is_sky_flat(&back_sector.floor_texture) {
            self.sky_quad(vertices, (min, floor));
        }

        let unpeg_upper = line.upper_unpegged();
        let back_floor = back_sector.floor_height;
        let back_ceil = back_sector.ceiling_height;
        let floor = if back_floor > floor {
            self.wall_quad(sector,
                           seg,
                           vertices,
                           (floor, back_floor),
                           &side.lower_texture,
                           if unpeg_lower {
                               Peg::BottomLower
                           } else {
                               Peg::Top
                           },
                           true);
            back_floor
        } else {
            floor
        };
        let ceil = if back_ceil < ceil {
            if !is_sky_flat(&back_sector.ceiling_texture) {
                self.wall_quad(sector,
                               seg,
                               vertices,
                               (back_ceil, ceil),
                               &side.upper_texture,
                               if unpeg_upper {
                                   Peg::Top
                               } else {
                                   Peg::Bottom
                               },
                               true);
            }
            back_ceil
        } else {
            ceil
        };
        self.wall_quad(sector,
                       seg,
                       vertices,
                       (floor, ceil),
                       &side.middle_texture,
                       if unpeg_lower {
                           if is_untextured(&side.upper_texture) {
                               Peg::TopFloat
                           } else {
                               Peg::Bottom
                           }
                       } else {
                           if is_untextured(&side.lower_texture) {
                               Peg::BottomFloat
                           } else {
                               Peg::Top
                           }
                       },
                       line.impassable());
    }

    fn wall_quad(&mut self,
                 sector: &WadSector,
                 seg: &WadSeg,
                 (v1, v2): (Vec2f, Vec2f),
                 (low, high): (WadCoord, WadCoord),
                 texture_name: &WadName,
                 peg: Peg,
                 blocking: bool) {
        if low >= high {
            return;
        }
        let size = if is_untextured(texture_name) {
            None
        } else if let Some(image) = self.tex.texture(texture_name) {
            Some(Vec2f::new(image.width() as f32, image.height() as f32))
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
        let side = if let Some(side) = self.level.seg_sidedef(seg) {
            side
        } else {
            warn!("Missing sidedef for seg, skipping wall.");
            return;
        };
        let bias = (v2 - v1).normalized() * POLY_BIAS;
        let (v1, v2) = (v1 - bias, v2 + bias);
        let (low, high) = match (size, peg) {
            (Some(size), Peg::TopFloat) => (from_wad_height(low + side.y_offset),
                                            from_wad_height(low + size[1] as i16 + side.y_offset)),
            (Some(size), Peg::BottomFloat) =>
                (from_wad_height(high + side.y_offset - size[1] as i16),
                 from_wad_height(high + side.y_offset)),
            _ => (from_wad_height(low), from_wad_height(high)),
        };

        let light_info_with_contrast;
        let light_info = light_info(&mut self.light_cache, &self.level, sector);
        let light_info = if light_info.effect.is_none() {
            if v1[0] == v2[0] {
                light_info_with_contrast = light::with_contrast(light_info, Contrast::Brighten);
                &light_info_with_contrast
            } else if v1[1] == v2[1] {
                light_info_with_contrast = light::with_contrast(light_info, Contrast::Darken);
                &light_info_with_contrast
            } else {
                light_info
            }
        } else {
            light_info
        };

        let height = (high - low) * 100.0;
        let s1 = seg.offset as f32 + side.x_offset as f32;
        let s2 = s1 + (v2 - v1).norm() * 100.0;
        let (t1, t2) = match (size, peg) {
            (Some(_), Peg::Top) | (None, _) => (height, 0.0),
            (Some(size), Peg::Bottom) => (size[1], size[1] - height),
            (Some(size), Peg::BottomLower) => {
                // As far as I can tell, this is a special case.
                let sector_height = (sector.ceiling_height - sector.floor_height) as f32;
                (size[1] + sector_height, size[1] - height + sector_height)
            }
            (Some(size), Peg::TopFloat) | (Some(size), Peg::BottomFloat) => (size[1], 0.0),
        };
        let (t1, t2) = (t1 + side.y_offset as f32, t2 + side.y_offset as f32);

        // TODO(cristicbz): Magic numbers below.
        let scroll = if line.special_type == 0x30 {
            35.0
        } else {
            0.0
        };

        let (low, high) = (low - POLY_BIAS, high + POLY_BIAS);

        self.visitor.visit_wall_quad(&(v1, v2),
                                     (s1, t1),
                                     (s2, t2),
                                     (low, high),
                                     light_info,
                                     scroll,
                                     size.map(|_| texture_name),
                                     blocking);
    }

    fn flat_poly(&mut self, sector: &WadSector) {
        let light_info = light_info(&mut self.light_cache, &self.level, sector);
        let (floor_tex, ceil_tex) = (&sector.floor_texture, &sector.ceiling_texture);
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
            self.visitor.visit_floor_sky_poly(&self.subsector_points, floor_y);
        } else {
            self.visitor.visit_floor_poly(&self.subsector_points, floor_y, light_info, floor_tex);
        }

        if ceil_sky {
            self.visitor.visit_ceil_sky_poly(&self.subsector_points, ceil_y);
        } else {
            self.visitor.visit_ceil_poly(&self.subsector_points, ceil_y, light_info, ceil_tex);
        }
    }

    fn sky_quad(&mut self, (v1, v2): (Vec2f, Vec2f), (low, high): (WadCoord, WadCoord)) {
        if low >= high {
            return;
        }
        let bias = (v2 - v1).normalized() * POLY_BIAS;
        let (v1, v2) = (v1 - bias, v2 + bias);
        let (low, high) = (from_wad_height(low), from_wad_height(high));

        self.visitor.visit_sky_quad(&(v1, v2), (low, high));
    }

    fn things(&mut self) {
        for thing in &self.level.things {
            let pos = from_wad_coords(thing.x, thing.y);
            let sector = match self.sector_at(&pos) {
                Some(sector) => sector,
                None => continue,
            };

            if let Some(marker) = Marker::from(thing.thing_type) {
                let pos = Vec3f::new(pos[0], from_wad_height(sector.floor_height), pos[1]);
                self.visitor.visit_marker(pos, marker);
            } else if let Some(sector) = self.sector_at(&pos) {
                self.decor(thing, &pos, sector);
            }
        }
    }

    fn sector_at(&self, pos: &Vec2f) -> Option<&'a WadSector> {
        let mut child_id = (self.level.nodes.len() - 1) as ChildId;
        loop {
            let (id, is_leaf) = parse_child_id(child_id);
            if is_leaf {
                let segs = self.level
                               .ssector(id)
                               .and_then(|subsector| self.level.ssector_segs(subsector))
                               .and_then(|segs| {
                                   if segs.is_empty() {
                                       None
                                   } else {
                                       Some(segs)
                                   }
                               });
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
                return if segs.iter()
                              .filter_map(|seg| self.level.seg_vertices(seg))
                              .map(|(v1, v2)| Line2f::from_two_points(v1, v2))
                              .all(|line| line.signed_distance(pos) <= SEG_TOLERANCE) {
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
                let partition = Line2f::from_origin_and_displace(from_wad_coords(node.line_x,
                                                                                 node.line_y),
                                                                 from_wad_coords(node.step_x,
                                                                                 node.step_y));
                if partition.signed_distance(pos) > 0.0f32 {
                    child_id = node.left;
                } else {
                    child_id = node.right;
                }
            }
        }
    }

    fn decor(&mut self, thing: &WadThing, pos: &Vec2f, sector: &WadSector) {
        let meta = match self.meta.find_thing(thing.thing_type) {
            Some(m) => m,
            None => {
                warn!("No metadata found for thing type {}", thing.thing_type);
                return;
            }
        };
        let (name, size) = {
            let mut s = meta.sprite.as_bytes().to_owned();
            s.push(meta.sequence.as_bytes()[0]);
            s.push(b'0');
            let n1 = WadName::from_bytes(&s);
            s.pop();
            s.push(b'1');
            let n2 = WadName::from_bytes(&s);

            match (n1, n2) {
                (Ok(n1), Ok(n2)) => {
                    if let Some(image) = self.tex.texture(&n1) {
                        (n1, image.size())
                    } else if let Some(image) = self.tex.texture(&n2) {
                        (n2, image.size())
                    } else {
                        warn!("No such sprite {} for thing {}",
                              meta.sprite,
                              thing.thing_type);
                        return;
                    }
                }
                _ => {
                    warn!("Metadata sprite name ({}) for thing type {} is not a valid WadName.",
                          meta.sprite,
                          thing.thing_type);
                    return;
                }
            }
        };
        let size = Vec2f::new(size[0] as f32, size[1] as f32);

        // TODO(cristicbz): Get rid of / 100.0 below.
        let (low, high) = if meta.hanging {
            (Vec3f::new(pos[0],
                        (sector.ceiling_height as f32 - size[1]) / 100.0,
                        pos[1]),
             Vec3f::new(pos[0], sector.ceiling_height as f32 / 100.0, pos[1]))
        } else {
            (Vec3f::new(pos[0], sector.floor_height as f32 / 100.0, pos[1]),
             Vec3f::new(pos[0],
                        (sector.floor_height as f32 + size[1]) / 100.0,
                        pos[1]))
        };
        let half_width = size[0] / 100.0 * 0.5;

        self.visitor.visit_decor(&low,
                                 &high,
                                 half_width,
                                 light_info(&mut self.light_cache, &self.level, sector),
                                 &name);
    }
}

fn light_info<'a>(cache: &'a mut VecMap<LightInfo>,
                  level: &Level,
                  sector: &WadSector)
                  -> &'a LightInfo {
    cache.entry(level.sector_id(sector) as usize)
         .or_insert_with(|| light::new_light(level, sector))
}

fn partition_line(node: &WadNode) -> Line2f {
    Line2f::from_origin_and_displace(from_wad_coords(node.line_x, node.line_y),
                                     from_wad_coords(node.step_x, node.step_y))
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
    let (min, max) = level.sectors
                          .iter()
                          .map(|s| (s.floor_height, s.ceiling_height))
                          .fold((32767, -32768),
                                |(min, max), (f, c)| (cmp::min(min, f), cmp::max(max, c)));
    (min, max + 32)
}

fn polygon_center(points: &[Vec2f]) -> Vec2f {
    let mut center = Vec2f::zero();
    for p in points.iter() {
        center = center + *p;
    }
    center / (points.len() as f32)
}

fn points_to_polygon(points: &mut Vec<Vec2f>) {
    // Sort points in polygonal CCW order around their center.
    let center = polygon_center(points);
    points.sort_by(|a, b| {
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

        if ac.cross(&bc) < 0.0 {
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
        let new_area = (next_point - current_point).cross(&(current_point - prev_point)) * 0.5;
        if new_area >= 0.0 {
            if area + new_area > 1.024e-05 {
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
    while (simplified[0] - simplified[simplified.len() - 1]).norm() < 0.0032 {
        simplified.pop();
    }

    let center = polygon_center(&simplified);
    for point in &mut simplified {
        *point = *point + (*point - center).normalized() * POLY_BIAS;
    }
    *points = simplified;
}


pub struct VisitorChain<'a, 'b, A: LevelVisitor + 'a, B: LevelVisitor + 'b> {
    first: &'a mut A,
    second: &'b mut B,
}

impl<'a, 'b, A: LevelVisitor, B: LevelVisitor> LevelVisitor for VisitorChain<'a, 'b, A, B> {
    fn visit_wall_quad(&mut self,
                       vertices: &(Vec2f, Vec2f),
                       tex_start: (f32, f32),
                       tex_end: (f32, f32),
                       height_range: (f32, f32),
                       light_info: &LightInfo,
                       scroll: f32,
                       tex_name: Option<&WadName>,
                       blocking: bool) {
        self.first.visit_wall_quad(vertices,
                                   tex_start,
                                   tex_end,
                                   height_range,
                                   light_info,
                                   scroll,
                                   tex_name,
                                   blocking);
        self.second.visit_wall_quad(vertices,
                                    tex_start,
                                    tex_end,
                                    height_range,
                                    light_info,
                                    scroll,
                                    tex_name,
                                    blocking);
    }

    fn visit_floor_poly(&mut self,
                        points: &[Vec2f],
                        height: f32,
                        light_info: &LightInfo,
                        tex_name: &WadName) {
        self.first.visit_floor_poly(points, height, light_info, tex_name);
        self.second.visit_floor_poly(points, height, light_info, tex_name);
    }

    fn visit_ceil_poly(&mut self,
                       points: &[Vec2f],
                       height: f32,
                       light_info: &LightInfo,
                       tex_name: &WadName) {
        self.first.visit_ceil_poly(points, height, light_info, tex_name);
        self.second.visit_ceil_poly(points, height, light_info, tex_name);
    }

    fn visit_floor_sky_poly(&mut self, points: &[Vec2f], height: f32) {
        self.first.visit_floor_sky_poly(points, height);
        self.second.visit_floor_sky_poly(points, height);
    }

    fn visit_ceil_sky_poly(&mut self, points: &[Vec2f], height: f32) {
        self.first.visit_ceil_sky_poly(points, height);
        self.second.visit_ceil_sky_poly(points, height);
    }

    fn visit_sky_quad(&mut self, vertices: &(Vec2f, Vec2f), height_range: (f32, f32)) {
        self.first.visit_sky_quad(vertices, height_range);
        self.second.visit_sky_quad(vertices, height_range);
    }

    fn visit_marker(&mut self, pos: Vec3f, marker: Marker) {
        self.first.visit_marker(pos, marker);
        self.second.visit_marker(pos, marker);
    }

    fn visit_decor(&mut self,
                   low: &Vec3f,
                   high: &Vec3f,
                   half_width: f32,
                   light_info: &LightInfo,
                   tex_name: &WadName) {
        self.first.visit_decor(low, high, half_width, light_info, tex_name);
        self.second.visit_decor(low, high, half_width, light_info, tex_name);
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


const THING_TYPE_PLAYER1_START: ThingType = 1;
const THING_TYPE_PLAYER2_START: ThingType = 2;
const THING_TYPE_PLAYER3_START: ThingType = 3;
const THING_TYPE_PLAYER4_START: ThingType = 4;
const THING_TYPE_TELEPORT_START: ThingType = 11;
const THING_TYPE_TELEPORT_END: ThingType = 14;

impl Marker {
    fn from(thing_type: ThingType) -> Option<Marker> {
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
