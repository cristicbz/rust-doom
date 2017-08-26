use super::lights::LightBuffer;
use super::world::World;
use gfx::{Scene, SceneBuilder};
use math::Vec3f;
use num::Zero;
use std::error::Error;
use wad::{Decor, LevelVisitor, LevelWalker, LightInfo, Marker, SkyPoly, SkyQuad, StaticPoly};
use wad::{SkyMetadata, TextureDirectory, WadMetadata};
use wad::Archive;
use wad::Level as WadLevel;
use wad::StaticQuad;
use wad::tex::{OpaqueImage, TransparentImage};
use wad::tex::BoundsLookup;
use wad::types::WadName;
use wad::util::{is_sky_flat, is_untextured};

pub struct Level {
    start_pos: Vec3f,
    time: f32,
    lights: LightBuffer,
    volume: World,
}

impl Level {
    pub fn new(
        wad: &Archive,
        textures: &TextureDirectory,
        level_index: usize,
        scene: &mut SceneBuilder,
    ) -> Result<Level, Box<Error>> {
        let name = *wad.level_name(level_index);
        info!("Building level {}...", name);
        let level = WadLevel::from_archive(wad, level_index)?;

        let palette = textures.build_palette_texture(0, 0, 32);
        scene.palette(&palette.pixels)?;

        scene.sky_program("sky")?;
        scene.static_program("static")?;
        scene.sprite_program("sprite")?;
        load_sky_texture(wad.metadata().sky_for(&name), textures, scene)?;

        let texture_maps = TextureMaps {
            flats: build_flats_atlas(&level, textures, scene)?,
            walls: build_walls_atlas(&level, textures, scene)?,
            decors: build_decor_atlas(&level, wad, textures, scene)?,
        };

        let mut volume = World::new();
        let mut lights = LightBuffer::new();
        let mut start_pos = Vec3f::zero();
        LevelBuilder::build(
            &level,
            &wad.metadata(),
            textures,
            &texture_maps,
            &mut start_pos,
            &mut lights,
            &mut volume,
            scene,
        );

        Ok(Level {
            start_pos: start_pos,
            time: 0.0,
            lights: lights,
            volume: volume,
        })
    }

    pub fn start_pos(&self) -> &Vec3f {
        &self.start_pos
    }

    pub fn volume(&self) -> &World {
        &self.volume
    }

    pub fn update(&mut self, delta_time: f32, scene: &mut Scene) {
        self.time += delta_time;
        scene.set_lights(|lights| self.lights.fill_buffer_at(self.time, lights))
    }
}


pub struct TextureMaps {
    flats: BoundsLookup,
    walls: BoundsLookup,
    decors: BoundsLookup,
}

fn load_sky_texture(
    meta: Option<&SkyMetadata>,
    textures: &TextureDirectory,
    scene: &mut SceneBuilder,
) -> Result<(), Box<Error>> {
    if let Some((Some(image), band)) =
        meta.map(|m| (textures.texture(&m.texture_name), m.tiled_band_size))
    {
        scene.tiled_band_size(band).sky_texture(
            image.pixels(),
            image.size(),
        )?;
    } else {
        warn!("Sky texture not found, will not render skies.");
        scene.no_sky_texture()?.tiled_band_size(0.0f32);
    }
    Ok(())
}

fn build_flats_atlas(
    level: &WadLevel,
    textures: &TextureDirectory,
    scene: &mut SceneBuilder,
) -> Result<BoundsLookup, Box<Error>> {
    let flat_name_iter = level
        .sectors
        .iter()
        .flat_map(|s| {
            Some(&s.floor_texture).into_iter().chain(
                Some(&s.ceiling_texture)
                    .into_iter(),
            )
        })
        .filter(|name| !is_untextured(*name) && !is_sky_flat(*name));
    let (OpaqueImage { pixels, size }, lookup) = textures.build_flat_atlas(flat_name_iter);
    scene.flats_texture(&pixels, size)?;
    Ok(lookup)
}

fn build_walls_atlas(
    level: &WadLevel,
    textures: &TextureDirectory,
    scene: &mut SceneBuilder,
) -> Result<BoundsLookup, Box<Error>> {
    let tex_name_iter = level
        .sidedefs
        .iter()
        .flat_map(|s| {
            Some(&s.upper_texture)
                .into_iter()
                .chain(Some(&s.lower_texture).into_iter())
                .chain(Some(&s.middle_texture).into_iter())
        })
        .filter(|name| !is_untextured(*name));
    let (TransparentImage { pixels, size }, lookup) = textures.build_texture_atlas(tex_name_iter);
    scene.walls_texture(&pixels, size)?;
    Ok(lookup)
}

fn build_decor_atlas(
    level: &WadLevel,
    archive: &Archive,
    textures: &TextureDirectory,
    scene: &mut SceneBuilder,
) -> Result<BoundsLookup, Box<Error>> {
    let tex_names = level
        .things
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
    scene.decors_texture(&pixels, size)?;
    Ok(lookup)
}

struct LevelBuilder<'a, 'b: 'a> {
    bounds: &'a TextureMaps,
    lights: &'a mut LightBuffer,
    scene: &'a mut SceneBuilder<'b>,
    start_pos: &'a mut Vec3f,
}

impl<'a, 'b: 'a> LevelBuilder<'a, 'b> {
    fn build(
        level: &WadLevel,
        meta: &WadMetadata,
        tex: &TextureDirectory,
        bounds: &TextureMaps,
        start_pos: &mut Vec3f,
        lights: &mut LightBuffer,
        volume: &mut World,
        scene: &mut SceneBuilder,
    ) {
        let mut builder = LevelBuilder {
            bounds: bounds,
            lights: lights,
            scene: scene,
            start_pos: start_pos,
        };
        LevelWalker::new(level, tex, meta, &mut builder.chain(volume)).walk();
    }

    fn add_light_info(&mut self, light_info: &LightInfo) -> u8 {
        self.lights.push(light_info)
    }
}


impl<'a, 'b: 'a> LevelVisitor for LevelBuilder<'a, 'b> {
    // TODO(cristicbz): Change some types here and unify as much as possible.
    fn visit_wall_quad(&mut self, quad: &StaticQuad) {
        let &StaticQuad {
            tex_name,
            light_info,
            scroll,
            vertices: &(ref v1, ref v2),
            height_range: (low, high),
            tex_start: (s1, t1),
            tex_end: (s2, t2),
            ..
        } = quad;

        let tex_name = if let Some(tex_name) = tex_name {
            tex_name
        } else {
            return;
        };
        let bounds = if let Some(bounds) = self.bounds.walls.get(tex_name) {
            bounds
        } else {
            warn!("No such wall texture {}.", tex_name);
            return;
        };
        let light_info = self.add_light_info(light_info);
        self.scene
            .walls_buffer()
            .push(v1, low, s1, t1, light_info, scroll, bounds)
            .push(v2, low, s2, t1, light_info, scroll, bounds)
            .push(v1, high, s1, t2, light_info, scroll, bounds)
            .push(v2, low, s2, t1, light_info, scroll, bounds)
            .push(v2, high, s2, t2, light_info, scroll, bounds)
            .push(v1, high, s1, t2, light_info, scroll, bounds);
    }

    fn visit_floor_poly(&mut self,
&StaticPoly { vertices, height, light_info, tex_name }: &StaticPoly){
        let bounds = if let Some(bounds) = self.bounds.flats.get(tex_name) {
            bounds
        } else {
            warn!("No such floor texture {}.", tex_name);
            return;
        };
        let light_info = self.add_light_info(light_info);
        let v0 = vertices[0];
        for (v1, v2) in vertices.iter().zip(vertices.iter().skip(1)) {
            self.scene
                .flats_buffer()
                .push(&v0, height, light_info, bounds)
                .push(&v1, height, light_info, bounds)
                .push(&v2, height, light_info, bounds);
        }
    }

    fn visit_ceil_poly(&mut self,
&StaticPoly { vertices, height, light_info, tex_name }: &StaticPoly){
        let bounds = if let Some(bounds) = self.bounds.flats.get(tex_name) {
            bounds
        } else {
            warn!("No such ceiling texture {}.", tex_name);
            return;
        };
        let light_info = self.add_light_info(light_info);
        let v0 = vertices[0];
        for (v1, v2) in vertices.iter().zip(vertices.iter().skip(1)) {
            self.scene
                .flats_buffer()
                .push(&v2, height, light_info, bounds)
                .push(&v1, height, light_info, bounds)
                .push(&v0, height, light_info, bounds);
        }
    }

    fn visit_floor_sky_poly(&mut self, &SkyPoly { vertices, height }: &SkyPoly) {
        let v0 = vertices[0];
        for (v1, v2) in vertices.iter().skip(1).zip(vertices.iter().skip(2)) {
            self.scene
                .sky_buffer()
                .push(&v0, height)
                .push(&v1, height)
                .push(&v2, height);
        }
    }

    fn visit_ceil_sky_poly(&mut self, &SkyPoly { vertices, height }: &SkyPoly) {
        let v0 = vertices[0];
        for (v1, v2) in vertices.iter().skip(1).zip(vertices.iter().skip(2)) {
            self.scene
                .sky_buffer()
                .push(&v2, height)
                .push(&v1, height)
                .push(&v0, height);
        }
    }

    fn visit_sky_quad(&mut self, quad: &SkyQuad) {
        let &SkyQuad {
            vertices: &(ref v1, ref v2),
            height_range: (low, high),
        } = quad;
        self.scene
            .sky_buffer()
            .push(v1, low)
            .push(v2, low)
            .push(v1, high)
            .push(v2, low)
            .push(v2, high)
            .push(v1, high);
    }

    fn visit_marker(&mut self, pos: Vec3f, marker: Marker) {
        if let Marker::StartPos { player: 0 } = marker {
            *self.start_pos = pos + Vec3f::new(0.0, 0.5, 0.0)
        }
    }

fn visit_decor(&mut self, &Decor { low, high, half_width, light_info, tex_name }: &Decor){
        let light_info = self.add_light_info(light_info);
        let bounds = if let Some(bounds) = self.bounds.decors.get(tex_name) {
            bounds
        } else {
            warn!("No such decor texture {}.", tex_name);
            return;
        };
        self.scene
            .decors_buffer()
            .push(&low, -half_width, 0.0, bounds.size[1], bounds, light_info)
            .push(
                &low,
                half_width,
                bounds.size[0],
                bounds.size[1],
                bounds,
                light_info,
            )
            .push(&high, -half_width, 0.0, 0.0, bounds, light_info)
            .push(
                &low,
                half_width,
                bounds.size[0],
                bounds.size[1],
                bounds,
                light_info,
            )
            .push(&high, half_width, bounds.size[0], 0.0, bounds, light_info)
            .push(&high, -half_width, 0.0, 0.0, bounds, light_info);
    }
}
