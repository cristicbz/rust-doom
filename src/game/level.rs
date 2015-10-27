use gfx::{Scene, SceneBuilder};
use lights::{LightBuffer, FakeContrast};
use math::{Line2f, Vec2f, Vec3f, Vector};
use num::Zero;
use std::cmp;
use std::cmp::Ordering;
use std::error::Error;
use std::vec::Vec;
use wad::tex::BoundsLookup;
use wad::tex::{OpaqueImage, TransparentImage};
use wad::types::{WadSeg, WadCoord, WadSector, WadName, WadThing, ChildId, ThingType};
use wad::types::{LightLevel, SectorId, SectorType};
use wad::util::{from_wad_height, from_wad_coords, is_untextured, parse_child_id, is_sky_flat};
use wad::{WadMetadata, SkyMetadata, ThingMetadata, TextureDirectory};
use wad::Archive as Archive;
use wad::Level as WadLevel;

pub struct Level {
    start_pos: Vec3f,
    time: f32,
    lights: LightBuffer,
    volume: WorldVolume,
}

impl Level {
    pub fn new(wad: &Archive,
               textures: &TextureDirectory,
               level_index: usize,
               scene: &mut SceneBuilder) -> Result<Level, Box<Error>> {
        let name = *wad.level_name(level_index);
        info!("Building level {}...", name);
        let level = try!(WadLevel::from_archive(wad, level_index));

        let palette = textures.build_palette_texture(0, 0, 32);
        try!(scene.palette(&palette.pixels));

        try!(scene.sky_program("sky"));
        try!(scene.static_program("static"));
        try!(scene.sprite_program("sprite"));
        try!(load_sky_texture(wad.metadata().sky_for(&name), textures, scene));

        let texture_maps = TextureMaps {
            flats: try!(build_flats_atlas(&level, textures, scene)),
            walls: try!(build_walls_atlas(&level, textures, scene)),
            decors: try!(build_decor_atlas(&level, wad, textures, scene)),
        };

        let mut volume = WorldVolume::new();
        let mut lights = LightBuffer::new();
        LevelBuilder::build(&level, &wad.metadata(),
                            textures, &texture_maps, &mut lights, &mut volume, scene);

        let start_pos = level.things.iter()
            .find(|thing| thing.thing_type == 1)
            .map(|thing| from_wad_coords(thing.x, thing.y))
            .map(|pos| {
                let height = 0.5 + volume.sector_at(&pos)
                    .map(|sector| sector.floor)
                    .unwrap_or(0.0);
                Vec3f::new(pos[0], height, pos[1])
            })
            .unwrap_or(Vec3f::zero());

        Ok(Level {
            start_pos: start_pos,
            time: 0.0,
            lights: lights,
            volume: volume,
        })
    }

    pub fn start_pos(&self) -> &Vec3f { &self.start_pos }

    pub fn heights_at(&self, pos: &Vec2f) -> Option<(f32, f32)> {
        self.volume.sector_at(pos).map(|s| (s.floor, s.ceil))
    }

    pub fn render(&mut self, delta_time: f32, scene: &mut Scene) {
        self.time += delta_time;
        scene.set_lights(|lights| {
            self.lights.fill_buffer_at(self.time, lights);
        });
    }
}


pub struct TextureMaps {
    flats: BoundsLookup,
    walls: BoundsLookup,
    decors: BoundsLookup,
}

#[derive(Copy, Clone)]
enum Peg {
    Top,
    Bottom,
    BottomLower,
    TopFloat,
    BottomFloat
}

// Distance on the wrong side of a BSP and seg line allowed.
const BSP_TOLERANCE: f32 = 1e-3;
const SEG_TOLERANCE: f32 = 0.1;

// All polygons are `fattened' by this amount to fill in thin gaps between them.
const POLY_BIAS: f32 = 0.64 * 3e-4;

pub fn find_thing(meta: &WadMetadata, thing_type: ThingType) -> Option<&ThingMetadata> {
    meta.things.decorations.iter().find(|t| t.thing_type == thing_type)
        .or_else(|| meta.things.weapons.iter().find(|t| t.thing_type == thing_type))
        .or_else(|| meta.things.powerups.iter().find(|t| t.thing_type == thing_type))
        .or_else(|| meta.things.artifacts.iter().find(|t| t.thing_type == thing_type))
        .or_else(|| meta.things.ammo.iter().find(|t| t.thing_type == thing_type))
        .or_else(|| meta.things.keys.iter().find(|t| t.thing_type == thing_type))
        .or_else(|| meta.things.monsters.iter().find(|t| t.thing_type == thing_type))
}

fn load_sky_texture(meta: Option<&SkyMetadata>,
                    textures: &TextureDirectory,
                    scene: &mut SceneBuilder) -> Result<(), Box<Error>> {
    if let Some((Some(image), band)) = meta.map(|m| (textures.texture(&m.texture_name),
                                                     m.tiled_band_size)) {
        try!(scene.tiled_band_size(band)
                  .sky_texture(image.pixels(), image.size()));
    } else {
        warn!("Sky texture not found, will not render skies.");
        try!(scene.no_sky_texture()).tiled_band_size(0.0f32);
    };
    Ok(())
}

fn build_flats_atlas(level: &WadLevel,
                     textures: &TextureDirectory,
                     scene: &mut SceneBuilder) -> Result<BoundsLookup, Box<Error>> {
    let flat_name_iter = level.sectors
            .iter()
            .flat_map(|s| Some(&s.floor_texture).into_iter()
                                                .chain(Some(&s.ceiling_texture).into_iter()))
            .filter(|name| !is_untextured(*name) && !is_sky_flat(*name));
    let (OpaqueImage { pixels, size }, lookup) = textures.build_flat_atlas(flat_name_iter);
    try!(scene.flats_texture(&pixels, size));
    Ok(lookup)
}

fn build_walls_atlas(level: &WadLevel, textures: &TextureDirectory, scene: &mut SceneBuilder)
        -> Result<BoundsLookup, Box<Error>> {
    let tex_name_iter = level.sidedefs
            .iter()
            .flat_map(|s| Some(&s.upper_texture).into_iter()
                          .chain(Some(&s.lower_texture).into_iter())
                          .chain(Some(&s.middle_texture).into_iter()))
            .filter(|name| !is_untextured(*name));
    let (TransparentImage { pixels, size }, lookup) = textures.build_texture_atlas(tex_name_iter);
    try!(scene.walls_texture(&pixels, size));
    Ok(lookup)
}

fn build_decor_atlas(level: &WadLevel,
                     archive: &Archive,
                     textures: &TextureDirectory,
                     scene: &mut SceneBuilder) -> Result<BoundsLookup, Box<Error>> {
    let tex_names = level.things
            .iter()
            .filter_map(|t| find_thing(archive.metadata(), t.thing_type))
            .flat_map(|d| {
                let mut s = d.sprite.as_bytes().to_owned();
                s.push(d.sequence.as_bytes()[0]);
                s.push(b'0');
                let n1 = WadName::from_bytes(&s);
                s.pop();
                s.push(b'1');
                let n2 = WadName::from_bytes(&s);
                n1.into_iter().chain(n2)
            })
            .filter(|name| !is_untextured(&name))
            .collect::<Vec<_>>();
    let (TransparentImage { pixels, size }, lookup) =
        textures.build_texture_atlas(tex_names.iter());
    try!(scene.decors_texture(&pixels, size));
    Ok(lookup)
}

pub struct Poly {
    sector: usize,
    poly: Vec<Vec2f>,
}

#[derive(Copy, Clone)]
pub struct Sector {
    floor: f32,
    ceil: f32,
    light_info: u8,
}


impl Poly {
    pub fn contains(&self, point: &Vec2f) -> bool {
        if self.poly.len() < 3 {
            return false;
        }
        self.poly.iter()
            .zip(self.poly[1..].iter().chain(Some(&self.poly[0]).into_iter()))
            .map(|(a, b)| Line2f::from_two_points(*a, *b))
            .all(|l| l.signed_distance(point) >= 0.0)
    }
}

pub struct WorldVolume {
    polys: Vec<Poly>,
    sectors: Vec<Option<Sector>>,
}
impl WorldVolume {
    pub fn new() -> WorldVolume {
        WorldVolume {
            polys: vec![],
            sectors: vec![],
        }
    }

    pub fn sector(&self, index: usize) -> Option<&Sector> {
        match self.sectors.get(index) {
            Some(sector) => sector.as_ref(),
            None => None,
        }
    }

    pub fn insert_sector(&mut self, index: usize, sector: Sector) {
        while self.sectors.len() <= index {
            self.sectors.push(None);
        }
        self.sectors[index] = Some(sector);
    }

    pub fn push_poly(&mut self, points: Vec<Vec2f>, sector_index: usize) {
        self.polys.push(Poly {
            poly: points,
            sector: sector_index,
        });
    }

    pub fn sector_at(&self, position: &Vec2f) -> Option<&Sector> {
        self.polys.iter()
            .find(|poly| poly.contains(position))
            .and_then(|poly| self.sector(poly.sector))
    }
}


struct LevelBuilder<'a, 'b: 'a> {
    bounds: &'a TextureMaps,
    lights: &'a mut LightBuffer,
    volume: &'a mut WorldVolume,
    scene: &'a mut SceneBuilder<'b>,
}
impl<'a, 'b: 'a> LevelBuilder<'a, 'b> {
    fn build(level: &WadLevel,
             meta: &WadMetadata,
             tex: &TextureDirectory,
             bounds: &TextureMaps,
             lights: &mut LightBuffer,
             volume: &mut WorldVolume,
             scene: &mut SceneBuilder) {
        let mut builder = LevelBuilder {
            bounds: bounds,
            lights: lights,
            volume: volume,
            scene: scene,
        };
        LevelWalker::new(level, tex, meta, &mut builder).walk();
    }

    fn add_light_info(&mut self, light_info: &LightInfo) -> u8 {
        self.lights.push(light_info.sector_light,
                         light_info.alt_light,
                         light_info.sector_type,
                         light_info.sector_id,
                         light_info.fake_contrast)
    }
}

fn polygon_center(points: &[Vec2f]) -> Vec2f {
    let mut center = Vec2f::zero();
    for p in points.iter() { center = center + *p; }
    center / (points.len() as f32)
}


fn points_to_polygon(points: &mut Vec<Vec2f>) {
    // Sort points in polygonal CCW order around their center.
    let center = polygon_center(points);
    points.sort_by(
        |a, b| {
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
                    }
                }
                return if b[1] > a[1] {
                    Ordering::Less
                } else {
                    Ordering::Greater
                }
            }

            if ac.cross(&bc) < 0.0 { Ordering::Less }
            else { Ordering::Greater }
        });

    // Remove duplicates.
    let mut simplified = Vec::new();
    simplified.push((*points)[0]);
    let mut current_point = (*points)[1];
    let mut area = 0.0;
    for i_point in 2..points.len() {
        let next_point = (*points)[i_point];
        let prev_point = simplified[simplified.len() - 1];
        let new_area = (next_point - current_point)
            .cross(&(current_point - prev_point)) * 0.5;
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
    if simplified.len() < 3 { points.clear(); return; }
    while (simplified[0] - simplified[simplified.len() - 1]).norm() < 0.0032 {
        simplified.pop();
    }

    let center = polygon_center(&simplified);
    for point in simplified.iter_mut() {
        *point = *point + (*point - center).normalized() * POLY_BIAS;
    }
    *points = simplified;
}


impl<'a, 'b: 'a> Visitor for LevelBuilder<'a, 'b> {
    // TODO(cristicbz): Change some types here and unify as much as possible.
    fn visit_wall_quad(&mut self,
                       &(ref v1, ref v2): &(Vec2f, Vec2f),
                       (s1, t1): (f32, f32),
                       (s2, t2): (f32, f32),
                       (low, high): (f32, f32),
                       light_info: &LightInfo,
                       scroll: f32,
                       tex_name: &WadName) {
        let bounds = if let Some(bounds) = self.bounds.walls.get(tex_name) { bounds } else {
            warn!("No such wall texture {}.", tex_name);
            return;
        };
        let light_info = self.add_light_info(light_info);
        self.scene.walls_buffer()
                  .push(v1, low,  s1, t1, light_info, scroll, bounds)
                  .push(v2, low,  s2, t1, light_info, scroll, bounds)
                  .push(v1, high, s1, t2, light_info, scroll, bounds)
                  .push(v2, low,  s2, t1, light_info, scroll, bounds)
                  .push(v2, high, s2, t2, light_info, scroll, bounds)
                  .push(v1, high, s1, t2, light_info, scroll, bounds);
    }

    fn visit_floor_poly(&mut self,
                        points: &[Vec2f],
                        height: f32,
                        light_info: &LightInfo,
                        tex_name: &WadName) {
        let bounds = if let Some(bounds) = self.bounds.flats.get(tex_name) { bounds } else {
            warn!("No such floor texture {}.", tex_name);
            return;
        };
        let light_info = self.add_light_info(light_info);
        let v0 = points[0];
        for i in 1..points.len() {
            let (v1, v2) = (points[i], points[(i + 1) % points.len()]);
            self.scene.flats_buffer()
                      .push(&v0, height, light_info, bounds)
                      .push(&v1, height, light_info, bounds)
                      .push(&v2, height, light_info, bounds);
        }
    }

    fn visit_ceil_poly(&mut self,
                       points: &[Vec2f],
                       height: f32,
                       light_info: &LightInfo,
                       tex_name: &WadName) {
        let bounds = if let Some(bounds) = self.bounds.flats.get(tex_name) { bounds } else {
            warn!("No such ceiling texture {}.", tex_name);
            return;
        };
        let light_info = self.add_light_info(light_info);
        let v0 = points[0];
        for i in 1..points.len() {
            let (v1, v2) = (points[i], points[(i + 1) % points.len()]);
            self.scene.flats_buffer()
                .push(&v2, height, light_info, bounds)
                .push(&v1, height, light_info, bounds)
                .push(&v0, height, light_info, bounds);
        }
    }

    fn visit_floor_sky_poly(&mut self, points: &[Vec2f], height: f32) {
        let v0 = points[0];
        for i in 1..points.len() {
            let (v1, v2) = (points[i], points[(i + 1) % points.len()]);
            self.scene.sky_buffer().push(&v0, height).push(&v1, height).push(&v2, height);
        }
    }

    fn visit_ceil_sky_poly(&mut self, points: &[Vec2f], height: f32) {
        let v0 = points[0];
        for i in 1..points.len() {
            let (v1, v2) = (points[i], points[(i + 1) % points.len()]);
            self.scene.sky_buffer().push(&v2, height).push(&v1, height).push(&v0, height);
        }
    }

    fn visit_sky_quad(&mut self, &(ref v1, ref v2): &(Vec2f, Vec2f), (low, high): (f32, f32)) {
        self.scene.sky_buffer()
            .push(v1, low).push(v2, low).push(v1, high)
            .push(v2, low).push(v2, high).push(v1, high);
    }

    fn visit_volume(&mut self,
                    sector_id: SectorId,
                    (floor, ceil): (f32, f32),
                    light_info: &LightInfo,
                    points: Vec<Vec2f>) {
        let sector_id = sector_id as usize;
        let light_info = self.add_light_info(light_info);
        if self.volume.sector(sector_id).is_none() {
            self.volume.insert_sector(sector_id,
                                      Sector {
                                          floor: floor,
                                          ceil: ceil,
                                          light_info: light_info,
                                      });
        }
        self.volume.push_poly(points, sector_id);
    }

    fn visit_decor(&mut self,
                   low: &Vec3f,
                   high: &Vec3f,
                   half_width: f32,
                   light_info: &LightInfo,
                   tex_name: &WadName) {
        let light_info = self.add_light_info(light_info);
        let bounds = if let Some(bounds) = self.bounds.decors.get(tex_name) { bounds } else {
            warn!("No such decor texture {}.", tex_name);
            return;
        };
        self.scene.decors_buffer()
            .push(&low, -half_width, 0.0, bounds.size[1], bounds, light_info)
            .push(&low, half_width, bounds.size[0], bounds.size[1], bounds, light_info)
            .push(&high, -half_width, 0.0, 0.0, bounds, light_info)
            .push(&low, half_width, bounds.size[0], bounds.size[1], bounds, light_info)
            .push(&high, half_width, bounds.size[0], 0.0, bounds, light_info)
            .push(&high, -half_width, 0.0, 0.0, bounds, light_info);
    }
}

pub trait Visitor {
    fn visit_wall_quad(&mut self,
                       vertices: &(Vec2f, Vec2f),
                       tex_start: (f32, f32),
                       tex_end: (f32, f32),
                       height_range: (f32, f32),
                       light_info: &LightInfo,
                       scroll: f32,
                       tex_name: &WadName);

    fn visit_floor_poly(&mut self,
                        points: &[Vec2f],
                        height: f32,
                        light_info: &LightInfo,
                        tex_name: &WadName);

    fn visit_ceil_poly(&mut self,
                       points: &[Vec2f],
                       height: f32,
                       light_info: &LightInfo,
                       tex_name: &WadName);

    fn visit_volume(&mut self,
                    sector_id: SectorId,
                    height_range: (f32, f32),
                    light_info: &LightInfo,
                    points: Vec<Vec2f>);

    fn visit_floor_sky_poly(&mut self, points: &[Vec2f], height: f32);
    fn visit_ceil_sky_poly(&mut self, points: &[Vec2f], height: f32);
    fn visit_sky_quad(&mut self, vertices: &(Vec2f, Vec2f), height_range: (f32, f32));

    fn visit_decor(&mut self,
                   low: &Vec3f,
                   high: &Vec3f,
                   half_width: f32,
                   light_info: &LightInfo,
                   tex_name: &WadName);
}


pub struct LevelWalker<'a, V: Visitor + 'a> {
    level: &'a WadLevel,
    tex: &'a TextureDirectory,
    meta: &'a WadMetadata,
    visitor: &'a mut V,
    height_range: HeightRange,
    bsp_lines: Vec<Line2f>,

    // The vector contains all (2D) points which are part of the subsector:
    // implicit (intersection of BSP lines) and explicit (seg vertices).
    subsector_points: Vec<Vec2f>,
    subsector_seg_lines: Vec<Line2f>,
}

impl<'a, V: Visitor> LevelWalker<'a, V> {
    pub fn new(level: &'a WadLevel,
               tex: &'a TextureDirectory,
               meta: &'a WadMetadata,
               visitor: &'a mut V) -> LevelWalker<'a, V> {
        LevelWalker {
            level: level,
            tex: tex,
            meta: meta,
            visitor: visitor,
            height_range: HeightRange::from(level),
            bsp_lines: Vec::with_capacity(32),
            subsector_points: Vec::with_capacity(32),
            subsector_seg_lines: Vec::with_capacity(32),
        }
    }

    pub fn walk(&mut self) {
        if self.level.nodes.is_empty() {
            warn!("Level contains no nodes, visitor not called at all.");
            return;
        }

        let root_id = (self.level.nodes.len() - 1) as ChildId;
        self.node(root_id);
        self.things();
    }

    fn node(&mut self, id: ChildId) {
        let (id, is_leaf) = parse_child_id(id);
        if is_leaf {
            self.subsector(id);
            return;
        }

        let node = if let Some(node) = self.level.nodes.get(id) { node }
            else {
                warn!("Missing entire node with id {}, skipping.", id);
                return;
            };
        let partition = Line2f::from_origin_and_displace(
            from_wad_coords(node.line_x, node.line_y),
            from_wad_coords(node.step_x, node.step_y));
        self.bsp_lines.push(partition);
        self.node(node.left);
        self.bsp_lines.pop();

        self.bsp_lines.push(partition.inverted_halfspaces());
        self.node(node.right);
        self.bsp_lines.pop();
    }

    fn subsector(&mut self, id: usize) {
        let subsector = if let Some(subsector) = self.level.ssector(id) { subsector } else {
            warn!("Cannot find subsector with id {}, will skip.", id);
            return;
        };
        let segs = if let Some(segs) = self.level.ssector_segs(subsector) { segs } else {
            warn!("Cannot find segs for subsector with id {}, will skip.", id);
            return;
        };
        if segs.is_empty() {
            warn!("Zero segs for subsector with id {}, will skip.", id);
            return;
        }
        let sector = if let Some(sector) = self.level.seg_sector(&segs[0]) { sector } else {
            warn!("Cannot find subsector with id {}, will skip.", id);
            return;
        };

        // These vectors get cleared for every subsector, we're just reusing the allocations.
        self.subsector_seg_lines.clear();
        self.subsector_seg_lines.reserve(segs.len());
        self.subsector_points.clear();
        self.subsector_points.reserve(segs.len() * 3);

        // First add the explicit points.
        for seg in segs {
            let (v1, v2) = if let Some(vertices) = self.level.seg_vertices(seg) { vertices } else {
                warn!("Cannot find seg vertices in subsector {}, will skip subsector.", id);
                return;
            };
            self.subsector_points.push(v1);
            self.subsector_points.push(v2);
            self.subsector_seg_lines.push(Line2f::from_two_points(v1, v2));

            // Also push the wall segments.
            self.seg(sector, seg, (v1, v2));
        }

        // The convex polyon defined at the intersection of the partition lines, intersected with
        // the half-volumes of the segs form the 'implicit' points.
        for i_line in 0..(self.bsp_lines.len() - 1) {
            for j_line in (i_line + 1)..self.bsp_lines.len() {
                let (l1, l2) = (&self.bsp_lines[i_line], &self.bsp_lines[j_line]);
                let point = match l1.intersect_point(l2) {
                    Some(p) => p,
                    None => continue
                };

                let dist = |l: &Line2f| l.signed_distance(&point);

                // The intersection point must lie both within the BSP volume
                // and the segs volume.
                if self.bsp_lines.iter().map(|x| dist(x)).all(|d| d >= -BSP_TOLERANCE)
                        && self.subsector_seg_lines.iter().map(dist).all(|d| d <= SEG_TOLERANCE) {
                    self.subsector_points.push(point);
                }
            }
        }
        if self.subsector_points.len() < 3 {
            warn!("Degenerate source polygon {} ({} vertices).", id, self.subsector_points.len());
        }
        points_to_polygon(&mut self.subsector_points);  // Sort and remove duplicates.
        if self.subsector_points.len() < 3 {
            warn!("Degenerate cannonicalised polygon {} ({} vertices).",
                  id, self.subsector_points.len());
        } else {
            self.flat_poly(sector);
        }
    }

    fn seg(&mut self, sector: &WadSector, seg: &WadSeg, vertices: (Vec2f, Vec2f)) {
        let line = if let Some(line) = self.level.seg_linedef(seg) { line }
            else {
                warn!("No linedef found for seg, skipping seg.");
                return;
            };
        let side = if let Some(side) = self.level.seg_sidedef(seg) { side }
            else {
                warn!("No sidedef found for seg, skipping seg.");
                return;
            };
        let (min, max) = (self.height_range.low, self.height_range.high);
        let (floor, ceil) = (sector.floor_height, sector.ceiling_height);
        let unpeg_lower = line.lower_unpegged();
        let back_sector = match self.level.seg_back_sector(seg) {
            None => {
                self.wall_quad(sector, seg, vertices, (floor, ceil), &side.middle_texture,
                               if unpeg_lower { Peg::Bottom } else { Peg::Top });
                if is_sky_flat(&sector.ceiling_texture) {
                    self.sky_quad(vertices, (ceil, max));
                }
                if is_sky_flat(&sector.floor_texture) {
                    self.sky_quad(vertices, (min, floor));
                }
                return
            },
            Some(s) => s
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
            self.wall_quad(sector, seg, vertices, (floor, back_floor), &side.lower_texture,
                           if unpeg_lower { Peg::BottomLower } else { Peg::Top });
            back_floor
        } else {
            floor
        };
        let ceil = if back_ceil < ceil {
            if !is_sky_flat(&back_sector.ceiling_texture) {
                self.wall_quad(sector, seg, vertices, (back_ceil, ceil), &side.upper_texture,
                               if unpeg_upper { Peg::Top } else { Peg::Bottom });
            }
            back_ceil
        } else {
            ceil
        };
        self.wall_quad(sector, seg, vertices, (floor, ceil), &side.middle_texture,
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
            });
    }

    fn wall_quad(&mut self,
                 sector: &WadSector,
                 seg: &WadSeg,
                 (v1, v2): (Vec2f, Vec2f),
                 (low, high): (WadCoord, WadCoord),
                 texture_name: &WadName,
                 peg: Peg) {
        if low >= high { return; }
        if is_untextured(texture_name) { return; }
        let size = if let Some(image) = self.tex.texture(texture_name) {
            Vec2f::new(image.width() as f32, image.height() as f32)
        } else {
            warn!("wall_quad: No such wall texture '{}'", texture_name);
            return
        };
        let line = if let Some(line) = self.level.seg_linedef(seg) { line } else {
            warn!("Missing linedef for seg, skipping wall.");
            return;
        };
        let side = if let Some(side) = self.level.seg_sidedef(seg) { side } else {
            warn!("Missing sidedef for seg, skipping wall.");
            return;
        };
        let bias = (v2 - v1).normalized() * POLY_BIAS;
        let (v1, v2) = (v1 - bias, v2 + bias);
        let (low, high) = match peg {
            Peg::TopFloat => (from_wad_height(low + side.y_offset),
                              from_wad_height(low + size[1] as i16 + side.y_offset)),
            Peg::BottomFloat => (from_wad_height(high + side.y_offset - size[1] as i16),
                                 from_wad_height(high + side.y_offset)),
            _ => (from_wad_height(low), from_wad_height(high))
        };

        let fake_contrast = if v1[0] == v2[0] {
            FakeContrast::Brighten
        } else if v1[1] == v2[1] {
            FakeContrast::Darken
        } else {
            FakeContrast::None
        };
        let light_info = self.light_info(sector, fake_contrast);
        let height = (high - low) * 100.0;
        let s1 = seg.offset as f32 + side.x_offset as f32;
        let s2 = s1 + (v2 - v1).norm() * 100.0;
        let (t1, t2) = match peg {
            Peg::Top => (height, 0.0),
            Peg::Bottom => (size[1], size[1] - height),
            Peg::BottomLower => {
                // As far as I can tell, this is a special case.
                let sector_height = (sector.ceiling_height - sector.floor_height) as f32;
                (size[1] + sector_height, size[1] - height + sector_height)
            },
            Peg::TopFloat | Peg::BottomFloat => (size[1], 0.0),
        };
        let (t1, t2) = (t1 + side.y_offset as f32, t2 + side.y_offset as f32);

        // TODO(cristicbz): Magic numbers below.
        let scroll = if line.special_type == 0x30 { 35.0 } else { 0.0 };

        let (low, high) = (low - POLY_BIAS, high + POLY_BIAS);
        self.visitor.visit_wall_quad(
            &(v1, v2), (s1, t1), (s2, t2), (low, high), &light_info, scroll, &texture_name);
    }

    fn flat_poly(&mut self, sector: &WadSector) {
        let light_info = self.light_info(sector, FakeContrast::None);
        let floor_y = from_wad_height(sector.floor_height);
        let floor_tex = &sector.floor_texture;
        let ceil_y = from_wad_height(sector.ceiling_height);
        let ceil_tex = &sector.ceiling_texture;

        // TODO(cristicbz): Re-add this somewhere.
        self.visitor.visit_volume(self.level.sector_id(sector),
                                  (floor_y,
                                   if is_sky_flat(ceil_tex) {
                                       from_wad_height(self.height_range.high)
                                   } else {
                                       ceil_y
                                   }),
                                  &light_info,
                                  self.subsector_points.clone());

        if !is_sky_flat(floor_tex) {
            self.visitor.visit_floor_poly(
                &self.subsector_points, floor_y, &light_info, floor_tex);
        } else {
            // TODO(cristicbz): investigate if we could store height_range in f32.
            self.visitor.visit_floor_sky_poly(&self.subsector_points,
                                              from_wad_height(self.height_range.low));
        }

        if !is_sky_flat(ceil_tex) {
            self.visitor.visit_ceil_poly(
                &self.subsector_points, ceil_y, &light_info, ceil_tex);
        } else {
            self.visitor.visit_ceil_sky_poly(&self.subsector_points,
                                             from_wad_height(self.height_range.high));
        }
    }

    fn sky_quad(&mut self, (v1, v2): (Vec2f, Vec2f), (low, high): (WadCoord, WadCoord)) {
        if low >= high { return; }
        let bias = (v2 - v1).normalized() * POLY_BIAS;
        let (v1, v2) = (v1 - bias, v2 + bias);
        let (low, high) = (from_wad_height(low), from_wad_height(high));

        self.visitor.visit_sky_quad(&(v1, v2), (low, high));
    }

    fn light_info(&mut self, sector: &WadSector, fake_contrast: FakeContrast) -> LightInfo {
        LightInfo {
            sector_light: sector.light,
            alt_light: self.level.sector_min_light(sector),
            sector_type: sector.sector_type,
            sector_id: self.level.sector_id(sector),
            fake_contrast: fake_contrast,
        }
    }

    fn things(&mut self) {
        for thing in self.level.things.iter() {
            let pos = from_wad_coords(thing.x, thing.y);
            if let Some(sector) = self.sector_at(&pos) {
                self.decor(thing, &pos, sector);
            }
        }
    }

    fn sector_at(&self, pos: &Vec2f) -> Option<&'a WadSector> {
        let mut child_id = (self.level.nodes.len() - 1) as ChildId;
        loop {
            let (id, is_leaf) = parse_child_id(child_id);
            if is_leaf {
                let segs = self.level.ssector(id)
                                     .and_then(|subsector| self.level.ssector_segs(subsector))
                                     .and_then(|segs| if segs.is_empty() {
                                                          None
                                                      } else {
                                                          Some(segs)
                                                      });
                let segs = if let Some(segs) = segs { segs } else {
                    return None;
                };
                let sector = if let Some(s) = self.level.seg_sector(&segs[0]) { s } else {
                    return None;
                };
                return if segs.iter()
                              .filter_map(|seg| self.level.seg_vertices(seg))
                              .map(|(v1, v2)| Line2f::from_two_points(v1, v2))
                              .all(|line| line.signed_distance(pos) <= SEG_TOLERANCE) {
                    Some(sector)
                } else {
                    None
                }
            } else {
                let node = if let Some(node) = self.level.nodes.get(id) { node } else {
                    return None;
                };
                let partition = Line2f::from_origin_and_displace(
                    from_wad_coords(node.line_x, node.line_y),
                    from_wad_coords(node.step_x, node.step_y));

                if partition.signed_distance(pos) > 0.0f32 {
                    child_id = node.left;
                } else {
                    child_id = node.right;
                }
            }
        }
    }

    fn decor(&mut self, thing: &WadThing, pos: &Vec2f, sector: &WadSector) {
        let meta = match find_thing(self.meta, thing.thing_type) {
            Some(m) => m,
            None => {
                warn!("No metadata found for thing type {}", thing.thing_type);
                return
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
                        warn!("No such sprite {} for thing {}", meta.sprite, thing.thing_type);
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
            (Vec3f::new(pos[0], (sector.ceiling_height as f32 - size[1]) / 100.0, pos[1]),
             Vec3f::new(pos[0], sector.ceiling_height as f32 / 100.0, pos[1]))
        } else {
            (Vec3f::new(pos[0], sector.floor_height as f32 / 100.0, pos[1]),
             Vec3f::new(pos[0], (sector.floor_height as f32 + size[1]) / 100.0, pos[1]))
        };
        let half_width = size[0] / 100.0 * 0.5;
        let light_info = self.light_info(sector, FakeContrast::None);

        self.visitor.visit_decor(&low, &high, half_width, &light_info, &name);
    }
}

pub struct LightInfo {
    pub sector_light: LightLevel,
    pub alt_light: LightLevel,
    // TODO(cristicbz): Replace this with LightEffect and own the conversion.
    pub sector_type: SectorType,
    // TODO(cristicbz): Decouple from sector id.
    pub sector_id: SectorId,
    // TODO(cristicbz): Own FakeContrast.
    pub fake_contrast: FakeContrast,
}

struct HeightRange {
    low: i16,
    high: i16,
}

impl HeightRange {
    fn from(level: &WadLevel) -> HeightRange {
        let (min, max) =
            level.sectors.iter()
                         .map(|s| (s.floor_height, s.ceiling_height))
                         .fold((32767, -32768),
                               |(min, max), (f, c)| (cmp::min(min, f), cmp::max(max, c)));
        HeightRange {
            low: min,
            high: max + 32,
        }
    }
}
