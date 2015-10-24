use gfx::{Scene, SceneBuilder};
use lights::{LightBuffer, FakeContrast};
use math::{Line2f, Vec2f, Vec3f, Vector};
use num::Zero;
use std::cmp::Ordering;
use std::error::Error;
use std::vec::Vec;
use wad;
use wad::tex::{BoundsLookup, TextureDirectory};
use wad::tex::{OpaqueImage, TransparentImage};
use wad::types::{WadSeg, WadCoord, WadSector, WadName, WadThing, ChildId, ThingType};
use wad::util::{from_wad_height, from_wad_coords, is_untextured, parse_child_id, is_sky_flat};
use wad::{WadMetadata, SkyMetadata, ThingMetadata};

pub struct Level {
    start_pos: Vec3f,
    time: f32,
    lights: LightBuffer,
    volume: WorldVolume,
}

impl Level {
    pub fn new(wad: &wad::Archive,
               textures: &TextureDirectory,
               level_index: usize,
               scene: &mut SceneBuilder) -> Result<Level, Box<Error>> {
        let name = *wad.level_name(level_index);
        info!("Building level {}...", name);
        let level = try!(wad::Level::from_archive(wad, level_index));

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
                            &texture_maps, &mut lights, &mut volume, scene);

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


struct TextureMaps {
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

macro_rules! offset_of(
    ($T:ty, $m:ident) => (
        unsafe { (&((*(0 as *const $T)).$m)) as *const _ as usize }
    )
);

fn load_sky_texture(meta: &wad::SkyMetadata,
                    textures: &wad::TextureDirectory,
                    scene: &mut SceneBuilder) -> Result<(), Box<Error>> {
    let image = textures.texture(&meta.texture_name).expect("Missing sky texture.");
    try!(scene.tiled_band_size(meta.tiled_band_size)
              .sky_texture(image.pixels(), image.size()));
    Ok(())
}


fn build_flats_atlas(level: &wad::Level,
                     textures: &wad::TextureDirectory,
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

fn build_walls_atlas(level: &wad::Level, textures: &wad::TextureDirectory, scene: &mut SceneBuilder)
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

fn build_decor_atlas(level: &wad::Level,
                     archive: &wad::Archive,
                     textures: &wad::TextureDirectory,
                     scene: &mut SceneBuilder) -> Result<BoundsLookup, Box<Error>> {
    let tex_names = level.things
            .iter()
            .filter_map(|t| find_thing(archive.metadata(), t.thing_type))
            .flat_map(|d| {
                let mut s = d.sprite.as_bytes().to_owned();
                s.push(d.sequence.as_bytes()[0]);
                s.push(b'0');
                let n1 = WadName::from_bytes(&s).unwrap();
                s.pop();
                s.push(b'1');
                let n2 = WadName::from_bytes(&s).unwrap();
                Some(n1).into_iter().chain(Some(n2).into_iter())
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
    level: &'a wad::Level,
    meta: &'a WadMetadata,
    bounds: &'a TextureMaps,
    lights: &'a mut LightBuffer,
    volume: &'a mut WorldVolume,
    scene: &'a mut SceneBuilder<'b>,
    min_height: i16,
    max_height: i16,
}
impl<'a, 'b: 'a> LevelBuilder<'a, 'b> {
    fn build(level: &wad::Level, meta: &WadMetadata,
             bounds: &TextureMaps, lights: &mut LightBuffer,
             volume: &mut WorldVolume, scene: &mut SceneBuilder) {
        let (min_height, max_height) = level.sectors
            .iter()
            .map(|s| (s.floor_height, s.ceiling_height))
            .fold((32767, -32768),
                  |(min, max), (f, c)| (if f < min { f } else { min },
                                        if c > max { c } else { max }));
        let max_height = max_height + 32;

        let mut builder = LevelBuilder {
            level: level,
            meta: meta,
            bounds: bounds,
            lights: lights,
            volume: volume,
            scene: scene,
            min_height: min_height,
            max_height: max_height,
        };
        let root_id = (level.nodes.len() - 1) as ChildId;
        builder.node(&mut Vec::with_capacity(32), root_id);
        builder.things();
    }

    fn things(&mut self) {
        for thing in self.level.things.iter() {
            let pos = from_wad_coords(thing.x, thing.y);
            if let Some(s) = self.volume.sector_at(&pos).map(|x| *x) {
                self.decor(thing, &pos, &s);
            }
        }
    }

    fn decor(&mut self, thing: &WadThing, pos: &Vec2f, sector: &Sector) {
        let meta = match find_thing(self.meta, thing.thing_type) {
            Some(m) => m,
            None => return,
        };
        let (name1, name2) = {
            let mut s = meta.sprite.as_bytes().to_owned();
            s.push(meta.sequence.as_bytes()[0]);
            s.push(b'0');
            let n1 = WadName::from_bytes(&s).unwrap();
            s.pop();
            s.push(b'1');
            let n2 = WadName::from_bytes(&s).unwrap();
            (n1, n2)
        };
        let bounds = if let Some(bounds) = self.bounds.decors.get(&name1)
                .or(self.bounds.decors.get(&name2)) {
            bounds
        } else {
            return;
        };

        let (low, high) = if meta.hanging {
            (Vec3f::new(pos[0], sector.ceil - bounds.size[1] / 100.0, pos[1]),
             Vec3f::new(pos[0], sector.ceil, pos[1]))
        } else {
            (Vec3f::new(pos[0], sector.floor, pos[1]),
             Vec3f::new(pos[0], sector.floor + bounds.size[1] / 100.0, pos[1]))
        };
        let half_width = bounds.size[0] / 100.0 * 0.5;

        self.scene.decors_buffer()
            .push(&low, -half_width, 0.0, bounds.size[1], bounds, sector.light_info)
            .push(&low, half_width, bounds.size[0], bounds.size[1], bounds, sector.light_info)
            .push(&high, -half_width, 0.0, 0.0, bounds, sector.light_info)
            .push(&low, half_width, bounds.size[0], bounds.size[1], bounds, sector.light_info)
            .push(&high, half_width, bounds.size[0], 0.0, bounds, sector.light_info)
            .push(&high, -half_width, 0.0, 0.0, bounds, sector.light_info);
    }

    fn node(&mut self, lines: &mut Vec<Line2f>, id: ChildId) {
        let (id, is_leaf) = parse_child_id(id);
        if is_leaf {
            self.subsector(lines, id);
            return;
        }

        let node = &self.level.nodes[id];
        let partition = Line2f::from_origin_and_displace(
            from_wad_coords(node.line_x, node.line_y),
            from_wad_coords(node.step_x, node.step_y));
        lines.push(partition);
        self.node(lines, node.left);
        lines.pop();

        lines.push(partition.inverted_halfspaces());
        self.node(lines, node.right);
        lines.pop();
    }

    fn subsector(&mut self, lines: &[Line2f], id: usize) {
        let segs = self.level.ssector_segs(&self.level.subsectors[id]);

        // The vector contains all (2D) points which are part of the subsector:
        // implicit (intersection of BSP lines) and explicit (seg vertices).
        let mut points = Vec::with_capacity(segs.len() * 3);
        let mut seg_lines = Vec::with_capacity(segs.len());

        // First add the explicit points.
        for seg in segs.iter() {
            let (v1, v2) = self.level.seg_vertices(seg);
            points.push(v1);
            points.push(v2);
            seg_lines.push(Line2f::from_two_points(v1, v2));

            // Also push the wall segments.
            self.seg(seg);
        }

        // The convex polyon defined at the intersection of the partition lines,
        // intersected with the half-volumes of the segs form the 'implicit'
        // points.
        for i_line in 0..(lines.len() - 1) {
            for j_line in (i_line + 1)..lines.len() {
                let (l1, l2) = (&(*lines)[i_line], &(*lines)[j_line]);
                let point = match l1.intersect_point(l2) {
                    Some(p) => p,
                    None => continue
                };

                let dist = |l: &Line2f| l.signed_distance(&point);

                // The intersection point must lie both within the BSP volume
                // and the segs volume.
                if lines.iter().map(|x| dist(x)).all(|d| d >= -BSP_TOLERANCE)
                   && seg_lines.iter().map(dist).all(|d| d <= SEG_TOLERANCE) {
                    points.push(point);
                }
            }
        }
        if points.len() < 3 {
            warn!("Degenerate source polygon {} ({} vertices).",
                  id, points.len());
        }
        points_to_polygon(&mut points);  // Sort and remove duplicates.
        if points.len() < 3 {
            warn!("Degenerate cannonicalised polygon {} ({} vertices).",
                  id, points.len());
        } else {
            self.flat_poly(self.level.seg_sector(&segs[0]), &points);
        }
    }

    fn seg(&mut self, seg: &WadSeg) {
        let line = self.level.seg_linedef(seg);
        let side = self.level.seg_sidedef(seg);
        let sector = self.level.sidedef_sector(side);
        let (min, max) = (self.min_height, self.max_height);
        let (floor, ceil) = (sector.floor_height, sector.ceiling_height);
        let unpeg_lower = line.lower_unpegged();
        let back_sector = match self.level.seg_back_sector(seg) {
            None => {
                self.wall_quad(seg, (floor, ceil), &side.middle_texture,
                               if unpeg_lower { Peg::Bottom } else { Peg::Top });
                if is_sky_flat(&sector.ceiling_texture) {
                    self.sky_quad(seg, (ceil, max));
                }
                if is_sky_flat(&sector.floor_texture) {
                    self.sky_quad(seg, (min, floor));
                }
                return
            },
            Some(s) => s
        };

        if is_sky_flat(&sector.ceiling_texture) && !is_sky_flat(&back_sector.ceiling_texture) {
            self.sky_quad(seg, (ceil, max));
        }
        if is_sky_flat(&sector.floor_texture) && !is_sky_flat(&back_sector.floor_texture) {
            self.sky_quad(seg, (min, floor));
        }

        let unpeg_upper = line.upper_unpegged();
        let back_floor = back_sector.floor_height;
        let back_ceil = back_sector.ceiling_height;
        let floor = if back_floor > floor {
            self.wall_quad(seg, (floor, back_floor), &side.lower_texture,
                           if unpeg_lower { Peg::BottomLower } else { Peg::Top });
            back_floor
        } else {
            floor
        };
        let ceil = if back_ceil < ceil {
            if !is_sky_flat(&back_sector.ceiling_texture) {
                self.wall_quad(seg, (back_ceil, ceil), &side.upper_texture,
                               if unpeg_upper { Peg::Top } else { Peg::Bottom });
            }
            back_ceil
        } else {
            ceil
        };
        self.wall_quad(seg, (floor, ceil), &side.middle_texture,
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

    fn wall_quad(&mut self, seg: &WadSeg, (low, high): (WadCoord, WadCoord),
                 texture_name: &WadName, peg: Peg) {
        if low >= high { return; }
        if is_untextured(texture_name) { return; }
        let bounds = match self.bounds.walls.get(texture_name) {
            None => {
                panic!("wall_quad: No such wall texture '{}'", texture_name);
            },
            Some(bounds) => bounds,
        };

        let line = self.level.seg_linedef(seg);
        let side = self.level.seg_sidedef(seg);
        let sector = self.level.sidedef_sector(side);
        let (v1, v2) = self.level.seg_vertices(seg);
        let bias = (v2 - v1).normalized() * POLY_BIAS;
        let (v1, v2) = (v1 - bias, v2 + bias);
        let (low, high) = match peg {
            Peg::TopFloat => (from_wad_height(low + side.y_offset),
                              from_wad_height(low + bounds.size[1] as i16 +
                                              side.y_offset)),
            Peg::BottomFloat => (from_wad_height(high + side.y_offset -
                                                 bounds.size[1] as i16),
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
            Peg::Bottom => (bounds.size[1], bounds.size[1] - height),
            Peg::BottomLower => {
                // As far as I can tell, this is a special case.
                let sector_height = (sector.ceiling_height -
                                     sector.floor_height) as f32;
                (bounds.size[1] + sector_height,
                 bounds.size[1] - height + sector_height)
            },
            Peg::TopFloat | Peg::BottomFloat => {
                (bounds.size[1], 0.0)
            }
        };
        let (t1, t2) = (t1 + side.y_offset as f32, t2 + side.y_offset as f32);

        let scroll = if line.special_type == 0x30 { 35.0 } else { 0.0 };

        let (low, high) = (low - POLY_BIAS, high + POLY_BIAS);
        self.scene.walls_buffer()
            .push(&v1, low,  s1, t1, light_info, scroll, bounds)
            .push(&v2, low,  s2, t1, light_info, scroll, bounds)
            .push(&v1, high, s1, t2, light_info, scroll, bounds)
            .push(&v2, low,  s2, t1, light_info, scroll, bounds)
            .push(&v2, high, s2, t2, light_info, scroll, bounds)
            .push(&v1, high, s1, t2, light_info, scroll, bounds);
    }

    fn flat_poly(&mut self, sector: &WadSector, points: &[Vec2f]) {
        let light_info = self.light_info(sector, FakeContrast::None);
        let floor_y = from_wad_height(sector.floor_height);
        let floor_tex = &sector.floor_texture;
        let ceil_y = from_wad_height(sector.ceiling_height);
        let ceil_tex = &sector.ceiling_texture;

        let sector_id = self.level.sector_id(sector) as usize;
        if let None = self.volume.sector(sector_id) {
            self.volume.insert_sector(sector_id, Sector {
                floor: floor_y,
                ceil: if is_sky_flat(ceil_tex) { from_wad_height(self.max_height) }
                      else { ceil_y },
                light_info: light_info,
            });
        }

        self.volume.push_poly(points.to_owned(), sector_id);

        let v0 = points[0];
        if !is_sky_flat(floor_tex) {
            let floor_bounds = self.bounds.flats
                .get(floor_tex)
                .expect(&format!("flat: No such floor {}.", floor_tex));
            for i in 1..points.len() {
                let (v1, v2) = (points[i], points[(i + 1) % points.len()]);
                self.scene.flats_buffer()
                    .push(&v0, floor_y, light_info, floor_bounds)
                    .push(&v1, floor_y, light_info, floor_bounds)
                    .push(&v2, floor_y, light_info, floor_bounds);
            }
        } else {
            let min = from_wad_height(self.min_height);
            for i in 1..points.len() {
                let (v1, v2) = (points[i], points[(i + 1) % points.len()]);

                self.scene.sky_buffer().push(&v0, min).push(&v1, min).push(&v2, min);
            }
        }

        if !is_sky_flat(ceil_tex) {
            let ceiling_bounds = self.bounds.flats
                .get(ceil_tex)
                .expect(&format!("flat: No such ceiling {}.", ceil_tex));
            for i in 1..points.len() {
                let (v1, v2) = (points[i], points[(i + 1) % points.len()]);
                self.scene.flats_buffer()
                    .push(&v2, ceil_y, light_info, ceiling_bounds)
                    .push(&v1, ceil_y, light_info, ceiling_bounds)
                    .push(&v0, ceil_y, light_info, ceiling_bounds);
            }
        } else {
            let max = from_wad_height(self.max_height);
            for i in 1..points.len() {
                let (v1, v2) = (points[i], points[(i + 1) % points.len()]);

                self.scene.sky_buffer().push(&v2, max).push(&v1, max).push(&v0, max);
            }
        }
    }

    fn sky_quad(&mut self, seg: &WadSeg, (low, high): (WadCoord, WadCoord)) {
        if low >= high { return; }
        let (v1, v2) = self.level.seg_vertices(seg);
        let bias = (v2 - v1).normalized() * POLY_BIAS;
        let (v1, v2) = (v1 - bias, v2 + bias);
        let (low, high) = (from_wad_height(low), from_wad_height(high));

        self.scene.sky_buffer().push(&v1, low).push(&v2, low).push(&v1, high);
        self.scene.sky_buffer().push(&v2, low).push(&v2, high).push(&v1, high);
    }

    fn light_info(&mut self, sector: &WadSector, fake_contrast: FakeContrast) -> u8 {
        self.lights.push(sector.light, self.level.sector_min_light(sector),
                         sector.sector_type, self.level.sector_id(sector),
                         fake_contrast)
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
        if new_area >= 0.0 && area + new_area > 1.024e-05 {
            area = 0.0;
            simplified.push(current_point);
        } else {
            area += new_area;
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
