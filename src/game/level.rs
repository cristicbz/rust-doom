use gfx::{Scene, SceneBuilder};
use lights::LightBuffer;
use math::{Line2f, Vec2f, Vec3f, Vector};
use num::Zero;
use std::error::Error;
use wad::tex::BoundsLookup;
use wad::tex::{OpaqueImage, TransparentImage};
use wad::types::{WadName, SectorId};
use wad::util::{from_wad_coords, is_untextured, is_sky_flat};
use wad::{WadMetadata, SkyMetadata, TextureDirectory};
use wad::Archive as Archive;
use wad::Level as WadLevel;
use wad::{LightInfo, LevelWalker, LevelVisitor};

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
            .filter_map(|t| archive.metadata().find_thing(t.thing_type))
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
        self.lights.push(light_info)
    }
}


impl<'a, 'b: 'a> LevelVisitor for LevelBuilder<'a, 'b> {
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
