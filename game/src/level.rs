use super::errors::{Error, Result};
use super::lights::Lights;
use super::vertex::{StaticVertex, SkyVertex, SpriteVertex};
use super::wad_system::WadSystem;
use super::world::World;
use engine::{Entities, Shaders, Uniforms, Materials, Meshes, Renderer, Window, ClientFormat,
             SamplerBehavior, SamplerWrapFunction, MinifySamplerFilter, MagnifySamplerFilter,
             Texture2dId, EntityId, PixelValue, FloatUniformId, MaterialId, BufferTextureId,
             BufferTextureType, ShaderId, System, Tick, Transforms};
use math::{Vec2, Vec3f, Vec2f};
use num::Zero;
use time;
use wad::{Decor, LevelVisitor, LevelWalker, LightInfo, Marker, SkyPoly, SkyQuad, StaticPoly};
use wad::Level as WadLevel;
use wad::StaticQuad;
use wad::tex::Bounds as WadBounds;
use wad::tex::BoundsLookup;
use wad::types::{WadName, PALETTE_SIZE, COLORMAP_SIZE};
use wad::util::{is_sky_flat, is_untextured};

pub struct Config {
    pub index: usize,
}

pub struct Level {
    root: EntityId,
    start_pos: Vec3f,
    start_yaw: f32,
    lights: Lights,
    volume: World,
    current_index: usize,
    next_index: usize,
    level_changed: bool,
    uniforms: DynamicUniforms,
}

derive_dependencies_from! {
    pub struct Dependencies<'context> {
        config: &'context Config,
        window: &'context Window,
        entities: &'context mut Entities,
        shaders: &'context mut Shaders,
        uniforms: &'context mut Uniforms,
        materials: &'context mut Materials,
        meshes: &'context mut Meshes,
        renderer: &'context mut Renderer,
        wad: &'context WadSystem,
        tick: &'context Tick,
        transforms: &'context mut Transforms,
    }
}


impl Level {
    pub fn level_index(&self) -> usize {
        self.current_index
    }

    pub fn change_level(&mut self, new_level_index: usize) {
        self.next_index = new_level_index;
    }

    pub fn level_changed(&self) -> bool {
        self.level_changed
    }

    pub fn root(&self) -> EntityId {
        self.root
    }

    pub fn start_pos(&self) -> &Vec3f {
        &self.start_pos
    }

    pub fn start_yaw(&self) -> f32 {
        self.start_yaw
    }

    pub fn volume(&self) -> &World {
        &self.volume
    }

    fn load(mut deps: Dependencies) -> Result<Self> {
        let root = deps.entities.add_root("level_root");

        let level_index = deps.config.index;
        ensure!(
            level_index < deps.wad.archive.num_levels(),
            "Level index {} is not in valid range 0..{}, see --list-levels for level names.",
            level_index,
            deps.wad.archive.num_levels()
        );
        let level_name = deps.level_name(level_index)?;
        info!("Loading level {}...", level_name);
        let wad_level = deps.load_wad_level(level_index)?;
        info!("Loading materials...");
        let material_data = deps.load_materials(root, &wad_level, level_name)?;

        info!("Building level...");
        let level = Builder::build(root, &mut deps, material_data, &wad_level)?;

        info!("Level {} loaded.", deps.config.index);
        Ok(level)
    }
}

impl<'context> System<'context> for Level {
    type Dependencies = Dependencies<'context>;
    type Error = Error;

    fn debug_name() -> &'static str {
        "level"
    }

    fn create(deps: Dependencies) -> Result<Self> {
        Self::load(deps)
    }

    fn update(&mut self, deps: Dependencies) -> Result<()> {
        if self.level_changed {
            info!("Level changed. {}", deps.entities.debug_tree_dump(4));
            self.level_changed = false;
        }

        if self.next_index != self.current_index {
            if self.next_index >= deps.wad.archive.num_levels() {
                self.next_index = self.current_index;
            } else {
                deps.entities.remove(self.root)?;
                *self = Self::create(Dependencies {
                    config: &Config { index: self.next_index },
                    ..deps
                })?;
                return Ok(());
            }
        }

        let time = {
            let time = deps.uniforms.get_float_mut(self.uniforms.time).expect(
                "time",
            );
            *time += deps.tick.timestep();
            *time
        };
        let light_infos = &mut self.lights;
        deps.uniforms.map_buffer_texture_u8(
            self.uniforms.lights_buffer_texture,
            |buffer| {
                light_infos.fill_buffer_at(time, buffer)
            },
        );
        Ok(())
    }

    fn teardown(&mut self, deps: Dependencies) -> Result<()> {
        let _ = deps.entities.remove(self.root);
        Ok(())
    }
}

impl<'context> Dependencies<'context> {
    fn load_palette(&mut self, parent: EntityId) -> Result<Texture2dId> {
        let palette = self.wad.textures.build_palette_texture(0, 0, 32);
        Ok(self.uniforms.add_texture_2d(
            self.window,
            self.entities,
            parent,
            "palette",
            &palette.pixels,
            Vec2::new(
                COLORMAP_SIZE,
                palette.pixels.len() / PALETTE_SIZE,
            ),
            ClientFormat::U8U8U8,
            Some(SamplerBehavior {
                wrap_function: (
                    SamplerWrapFunction::Clamp,
                    SamplerWrapFunction::Clamp,
                    SamplerWrapFunction::Clamp,
                ),
                minify_filter: MinifySamplerFilter::Nearest,
                magnify_filter: MagnifySamplerFilter::Nearest,
                max_anisotropy: 1,
            }),
        )?)
    }

    fn load_wad_texture<'b, T: PixelValue>(
        &mut self,
        parent: EntityId,
        name: &'static str,
        pixels: &'b [T],
        size: Vec2<usize>,
        format: ClientFormat,
    ) -> Result<Texture2dId> {
        Ok(self.uniforms.add_texture_2d(
            self.window,
            self.entities,
            parent,
            name,
            pixels,
            size,
            format,
            Some(SamplerBehavior {
                wrap_function: (
                    SamplerWrapFunction::Repeat,
                    SamplerWrapFunction::Repeat,
                    SamplerWrapFunction::Repeat,
                ),
                minify_filter: MinifySamplerFilter::Nearest,
                magnify_filter: MagnifySamplerFilter::Nearest,
                max_anisotropy: 1,
            }),
        )?)
    }

    fn load_flats_atlas(&mut self, parent: EntityId, wad_level: &WadLevel) -> Result<Atlas> {
        info!("Building flats atlas...");
        let names = wad_level
            .sectors
            .iter()
            .flat_map(|sector| {
                Some(sector.floor_texture).into_iter().chain(Some(
                    sector.ceiling_texture,
                ))
            })
            .filter(|name| !is_untextured(name) && !is_sky_flat(name));
        let (image, bounds) = self.wad.textures.build_flat_atlas(names);
        let texture = self.load_wad_texture(
            parent,
            "flats_atlas_texture",
            &image.pixels,
            image.size,
            ClientFormat::U8,
        )?;
        Ok(Atlas { texture, bounds })
    }

    fn load_walls_atlas(&mut self, parent: EntityId, wad_level: &WadLevel) -> Result<Atlas> {
        info!("Building walls atlas...");
        let names = wad_level
            .sidedefs
            .iter()
            .flat_map(|sidedef| {
                Some(sidedef.upper_texture)
                    .into_iter()
                    .chain(Some(sidedef.lower_texture))
                    .chain(Some(sidedef.middle_texture))
            })
            .filter(|name| !is_untextured(name));
        let (image, bounds) = self.wad.textures.build_texture_atlas(names);
        let texture = self.load_wad_texture(
            parent,
            "walls_atlas_texture",
            &image.pixels,
            image.size,
            ClientFormat::U8U8,
        )?;
        Ok(Atlas { texture, bounds })
    }

    fn load_decor_atlas(&mut self, parent: EntityId, wad_level: &WadLevel) -> Result<Atlas> {
        info!("Building sprite decorations atlas...");
        let (image, bounds) = {
            let wad = self.wad;
            let names = wad_level
                .things
                .iter()
                .filter_map(|thing| wad.archive.metadata().find_thing(thing.thing_type))
                .flat_map(|decor| {
                    let mut sprite0 = decor.sprite;
                    let _ = sprite0.push(decor.sequence.as_bytes()[0]);
                    let mut sprite1 = sprite0;
                    let sprite0 = sprite0.push(b'0').ok().map(|_| sprite0);
                    let sprite1 = sprite1.push(b'1').ok().map(|_| sprite1);
                    sprite0.into_iter().chain(sprite1)
                });
            wad.textures.build_texture_atlas(names)
        };
        let texture = self.load_wad_texture(
            parent,
            "decor_atlas_texture",
            &image.pixels,
            image.size,
            ClientFormat::U8U8,
        )?;
        Ok(Atlas { texture, bounds })
    }

    fn load_sky_uniforms(&mut self, parent: EntityId, level_name: WadName) -> Result<SkyUniforms> {
        let image_and_band = {
            let WadSystem {
                ref archive,
                ref textures,
            } = *self.wad;
            archive.metadata().sky_for(&level_name).map(|sky_metadata| {
                (
                    textures.texture(&sky_metadata.texture_name),
                    sky_metadata.tiled_band_size,
                )
            })
        };
        Ok(if let Some((Some(image), band)) = image_and_band {
            SkyUniforms {
                texture: self.load_wad_texture(
                    parent,
                    "sky_texture",
                    image.pixels(),
                    image.size(),
                    ClientFormat::U8U8,
                )?,
                tiled_band_size: self.uniforms.add_float(
                    self.entities,
                    parent,
                    "sky_tiled_band_size_uniform",
                    band,
                )?,
            }
        } else {
            warn!("Sky texture not found, will not render skies.");
            SkyUniforms {
                texture: self.load_wad_texture(
                    parent,
                    "sky_dummy_texture",
                    &[0u16],
                    Vec2::new(1, 1),
                    ClientFormat::U8U8,
                )?,
                tiled_band_size: self.uniforms.add_float(
                    self.entities,
                    parent,
                    "sky_tiled_band_size_dummy_uniform",
                    0.0,
                )?,
            }
        })
    }

    fn load_materials(
        &mut self,
        parent: EntityId,
        wad_level: &WadLevel,
        level_name: WadName,
    ) -> Result<MaterialData> {
        let palette = self.load_palette(parent)?;
        let time = self.uniforms.add_float(
            self.entities,
            parent,
            "time_uniform",
            0.0,
        )?;
        let lights_buffer_texture = self.uniforms.add_persistent_buffer_texture_u8(
            self.window,
            self.entities,
            parent,
            "lights_buffer_texture",
            256,
            BufferTextureType::Float,
        )?;

        let modelview = self.renderer.modelview();
        let projection = self.renderer.projection();

        let static_shader = self.load_shader(parent, "static_shader", "static")?;
        let flats_atlas = self.load_flats_atlas(parent, wad_level)?;
        let flats_material = self.materials
            .add(self.entities, static_shader, "flats_material")?
            .add_uniform("u_modelview", modelview)
            .add_uniform("u_projection", projection)
            .add_uniform("u_time", time)
            .add_uniform("u_lights", lights_buffer_texture)
            .add_uniform("u_palette", palette)
            .add_uniform("u_atlas", flats_atlas.texture)
            .add_uniform(
                "u_atlas_size",
                self.uniforms.add_texture2d_size(
                    self.entities,
                    "flats_atlas_size_uniform",
                    flats_atlas.texture,
                )?,
            )
            .id();
        let walls_atlas = self.load_walls_atlas(parent, wad_level)?;
        let walls_material = self.materials
            .add(self.entities, static_shader, "walls_material")?
            .add_uniform("u_modelview", modelview)
            .add_uniform("u_projection", projection)
            .add_uniform("u_time", time)
            .add_uniform("u_lights", lights_buffer_texture)
            .add_uniform("u_palette", palette)
            .add_uniform("u_atlas", walls_atlas.texture)
            .add_uniform(
                "u_atlas_size",
                self.uniforms.add_texture2d_size(
                    self.entities,
                    "walls_atlas_size_uniform",
                    walls_atlas.texture,
                )?,
            )
            .id();

        let sky_shader = self.load_shader(parent, "sky_shader", "sky")?;
        let sky_uniforms = self.load_sky_uniforms(parent, level_name)?;
        let sky_material = self.materials
            .add(self.entities, sky_shader, "sky_material")?
            .add_uniform("u_modelview", modelview)
            .add_uniform("u_projection", projection)
            .add_uniform("u_time", time)
            .add_uniform("u_palette", palette)
            .add_uniform("u_texture", sky_uniforms.texture)
            .add_uniform("u_tiled_band_size", sky_uniforms.tiled_band_size)
            .id();

        let sprite_shader = self.load_shader(parent, "sprite_shader", "sprite")?;
        let decor_atlas = self.load_decor_atlas(parent, wad_level)?;
        let decor_material = self.materials
            .add(self.entities, sprite_shader, "decor_material")?
            .add_uniform("u_modelview", modelview)
            .add_uniform("u_projection", projection)
            .add_uniform("u_time", time)
            .add_uniform("u_lights", lights_buffer_texture)
            .add_uniform("u_palette", palette)
            .add_uniform("u_atlas", decor_atlas.texture)
            .add_uniform(
                "u_atlas_size",
                self.uniforms.add_texture2d_size(
                    self.entities,
                    "decor_atlas_size_uniform",
                    decor_atlas.texture,
                )?,
            )
            .id();

        Ok(MaterialData {
            uniforms: DynamicUniforms {
                time,
                lights_buffer_texture,
            },
            materials: LevelMaterials {
                flats: AtlasMaterial {
                    material: flats_material,
                    bounds: flats_atlas.bounds,
                },
                walls: AtlasMaterial {
                    material: walls_material,
                    bounds: walls_atlas.bounds,
                },
                decor: AtlasMaterial {
                    material: decor_material,
                    bounds: decor_atlas.bounds,
                },
                sky: sky_material,
            },
        })
    }

    fn load_shader(
        &mut self,
        parent: EntityId,
        name: &'static str,
        asset: &'static str,
    ) -> Result<ShaderId> {
        Ok(self.shaders.add(
            self.window,
            self.entities,
            parent,
            name,
            asset,
        )?)
    }

    fn level_name(&self, index: usize) -> Result<WadName> {
        Ok(self.wad.archive.level_lump(index)?.name())
    }

    fn load_wad_level(&self, index: usize) -> Result<WadLevel> {
        Ok(WadLevel::from_archive(&self.wad.archive, index)?)
    }
}

struct SkyUniforms {
    tiled_band_size: FloatUniformId,
    texture: Texture2dId,
}

struct DynamicUniforms {
    time: FloatUniformId,
    lights_buffer_texture: BufferTextureId<u8>,
}

struct AtlasMaterial {
    material: MaterialId,
    bounds: BoundsLookup,
}

struct LevelMaterials {
    flats: AtlasMaterial,
    walls: AtlasMaterial,
    decor: AtlasMaterial,
    sky: MaterialId,
}

struct MaterialData {
    uniforms: DynamicUniforms,
    materials: LevelMaterials,
}

struct Atlas {
    texture: Texture2dId,
    bounds: BoundsLookup,
}


struct Builder<'a, 'context: 'a> {
    deps: &'a mut Dependencies<'context>,
    materials: LevelMaterials,
    uniforms: DynamicUniforms,

    lights: Lights,
    start_pos: Vec3f,
    start_yaw: f32,

    static_vertices: Vec<StaticVertex>,
    sky_vertices: Vec<SkyVertex>,
    decor_vertices: Vec<SpriteVertex>,

    wall_indices: Vec<u32>,
    flat_indices: Vec<u32>,
    sky_indices: Vec<u32>,
    decor_indices: Vec<u32>,

    num_wall_quads: usize,
    num_floor_polys: usize,
    num_ceil_polys: usize,
    num_sky_wall_quads: usize,
    num_sky_floor_polys: usize,
    num_sky_ceil_polys: usize,
    num_decors: usize,
}

impl<'a, 'context> Builder<'a, 'context> {
    fn build(
        root: EntityId,
        deps: &mut Dependencies<'context>,
        material_data: MaterialData,
        wad_level: &WadLevel,
    ) -> Result<Level> {
        let mut volume = World::new();
        let mut builder = Builder {
            deps,
            materials: material_data.materials,
            uniforms: material_data.uniforms,

            lights: Lights::new(),
            start_pos: Vec3f::zero(),
            start_yaw: 0.0f32,

            static_vertices: Vec::with_capacity(16_384),
            sky_vertices: Vec::with_capacity(16_384),
            decor_vertices: Vec::with_capacity(16_384),

            wall_indices: Vec::with_capacity(65_536),
            flat_indices: Vec::with_capacity(65_536),
            sky_indices: Vec::with_capacity(65_536),
            decor_indices: Vec::with_capacity(65_536),

            num_wall_quads: 0,
            num_floor_polys: 0,
            num_ceil_polys: 0,
            num_sky_wall_quads: 0,
            num_sky_floor_polys: 0,
            num_sky_ceil_polys: 0,
            num_decors: 0,
        };
        info!("Walking level...");
        let start_time = time::precise_time_s();
        LevelWalker::new(
            wad_level,
            &builder.deps.wad.textures,
            builder.deps.wad.archive.metadata(),
            &mut builder.chain(&mut volume),
        ).walk();
        info!(
            "Level built in {:.2}ms:\n\
            \tnum_wall_quads = {}\n\
            \tnum_floor_polys = {}\n\
            \tnum_ceil_polys = {}\n\
            \tnum_sky_wall_quads = {}\n\
            \tnum_sky_floor_polys = {}\n\
            \tnum_sky_ceil_polys = {}\n\
            \tnum_decors = {}\n\
            \tnum_static_tris = {}\n\
            \tnum_sky_tris = {}\n\
            \tnum_sprite_tris = {}",
            (time::precise_time_s() - start_time) * 1000.0,
            builder.num_wall_quads,
            builder.num_floor_polys,
            builder.num_ceil_polys,
            builder.num_sky_wall_quads,
            builder.num_sky_floor_polys,
            builder.num_sky_ceil_polys,
            builder.num_decors,
            (builder.wall_indices.len() + builder.flat_indices.len()) / 3,
            builder.sky_indices.len() / 3,
            builder.decor_indices.len() / 3
        );

        let deps = builder.deps;
        info!("Creating static meshes and models...");
        let statics = deps.entities.add(root, "statics")?;
        let static_mesh = deps.meshes.add_immutable::<_, u8>(
            deps.window,
            deps.entities,
            statics,
            "static_mesh",
            &builder.static_vertices,
            None,
        )?;
        let flats_mesh = deps.meshes.add_immutable_indices(
            deps.window,
            deps.entities,
            static_mesh,
            "flats_mesh",
            &builder.flat_indices,
        )?;
        let flats = deps.entities.add(statics, "flats")?;
        deps.transforms.attach_identity(flats);
        deps.renderer.attach_model(
            flats,
            flats_mesh,
            builder.materials.flats.material,
        )?;

        let walls_mesh = deps.meshes.add_immutable_indices(
            deps.window,
            deps.entities,
            static_mesh,
            "walls_mesh",
            &builder.wall_indices,
        )?;
        let walls = deps.entities.add(statics, "walls")?;
        deps.transforms.attach_identity(walls);
        deps.renderer.attach_model(
            walls,
            walls_mesh,
            builder.materials.walls.material,
        )?;

        let decor = deps.entities.add(statics, "decor")?;
        let decor_mesh = deps.meshes.add_immutable(
            deps.window,
            deps.entities,
            decor,
            "decor_mesh",
            &builder.decor_vertices,
            Some(&builder.decor_indices),
        )?;
        deps.transforms.attach_identity(decor);
        deps.renderer.attach_model(
            decor,
            decor_mesh,
            builder.materials.decor.material,
        )?;

        let sky = deps.entities.add(statics, "sky")?;
        let sky_mesh = deps.meshes.add_immutable(
            deps.window,
            deps.entities,
            sky,
            "sky_mesh",
            &builder.sky_vertices,
            Some(&builder.sky_indices),
        )?;
        deps.transforms.attach_identity(sky);
        deps.renderer.attach_model(
            sky,
            sky_mesh,
            builder.materials.sky,
        )?;

        Ok(Level {
            root,
            volume,
            start_pos: builder.start_pos,
            start_yaw: builder.start_yaw,
            lights: builder.lights,
            uniforms: builder.uniforms,
            current_index: deps.config.index,
            next_index: deps.config.index,
            level_changed: true,
        })
    }

    #[cfg_attr(feature = "cargo-clippy", allow(too_many_arguments))]
    fn wall_vertex(
        &mut self,
        xz: &Vec2f,
        y: f32,
        tile_u: f32,
        tile_v: f32,
        light_info: u8,
        scroll_rate: f32,
        bounds: &WadBounds,
    ) -> &mut Self {
        self.static_vertices.push(StaticVertex {
            a_pos: [xz[0], y, xz[1]],
            a_atlas_uv: [bounds.pos[0], bounds.pos[1]],
            a_tile_uv: [tile_u, tile_v],
            a_tile_size: [bounds.size[0], bounds.size[1]],
            a_scroll_rate: scroll_rate,
            a_num_frames: bounds.num_frames as u8,
            a_row_height: bounds.row_height as f32,
            a_light: light_info,
        });
        self
    }

    fn flat_vertex(&mut self, xz: &Vec2f, y: f32, light_info: u8, bounds: &WadBounds) -> &mut Self {
        self.static_vertices.push(StaticVertex {
            a_pos: [xz[0], y, xz[1]],
            a_atlas_uv: [bounds.pos[0], bounds.pos[1]],
            a_tile_uv: [-xz[0] * 100.0, -xz[1] * 100.0],
            a_tile_size: [bounds.size[0], bounds.size[1]],
            a_scroll_rate: 0.0,
            a_num_frames: bounds.num_frames as u8,
            a_row_height: bounds.row_height as f32,
            a_light: light_info,
        });
        self
    }

    fn sky_vertex(&mut self, xz: &Vec2f, y: f32) -> &mut Self {
        self.sky_vertices.push(
            SkyVertex { a_pos: [xz[0], y, xz[1]] },
        );
        self
    }

    fn decor_vertex(
        &mut self,
        pos: &Vec3f,
        local_x: f32,
        tile_u: f32,
        tile_v: f32,
        bounds: &WadBounds,
        light_info: u8,
    ) -> &mut Self {
        self.decor_vertices.push(SpriteVertex {
            a_pos: [pos[0], pos[1], pos[2]],
            a_local_x: local_x,
            a_atlas_uv: [bounds.pos[0], bounds.pos[1]],
            a_tile_uv: [tile_u, tile_v],
            a_tile_size: [bounds.size[0], bounds.size[1]],
            a_num_frames: 1,
            a_light: light_info,
        });
        self
    }

    fn flat_poly(&mut self, poly_length: usize) {
        Self::any_poly(
            self.static_vertices.len(),
            poly_length,
            &mut self.flat_indices,
        );
    }

    fn wall_quad(&mut self) {
        Self::any_quad(self.static_vertices.len(), &mut self.wall_indices);
    }

    fn sky_poly(&mut self, poly_length: usize) {
        Self::any_poly(self.sky_vertices.len(), poly_length, &mut self.sky_indices);
    }

    fn sky_quad(&mut self) {
        Self::any_quad(self.sky_vertices.len(), &mut self.sky_indices);
    }

    fn decor_quad(&mut self) {
        Self::any_quad(self.decor_vertices.len(), &mut self.decor_indices);
    }

    fn add_light_info(&mut self, light_info: &LightInfo) -> u8 {
        self.lights.push(light_info)
    }

    fn any_quad(new_length: usize, indices: &mut Vec<u32>) {
        let new_length = new_length as u32;
        let v0 = new_length - 4;
        let v1 = v0 + 1;
        let v2 = v1 + 1;
        let v3 = v2 + 1;

        indices.push(v0);
        indices.push(v1);
        indices.push(v3);

        indices.push(v1);
        indices.push(v2);
        indices.push(v3);
    }

    fn any_poly(new_length: usize, poly_length: usize, indices: &mut Vec<u32>) {
        let new_length = new_length as u32;
        let poly_length = poly_length as u32;
        let v0 = new_length - poly_length;
        for (v1, v2) in (v0..new_length).zip((v0 + 1)..new_length) {
            indices.push(v0);
            indices.push(v1);
            indices.push(v2);
        }
    }
}


impl<'a, 'context> LevelVisitor for Builder<'a, 'context> {
    // TODO(cristicbz): Change some types here and unify as much as possible.
    fn visit_wall_quad(&mut self, quad: &StaticQuad) {
        self.num_wall_quads += 1;
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
        let bounds = if let Some(bounds) = self.materials.walls.bounds.get(tex_name) {
            *bounds
        } else {
            warn!("No such wall texture {}.", tex_name);
            return;
        };
        let light_info = self.add_light_info(light_info);
        self.wall_vertex(v1, low, s1, t1, light_info, scroll, &bounds)
            .wall_vertex(v2, low, s2, t1, light_info, scroll, &bounds)
            .wall_vertex(v2, high, s2, t2, light_info, scroll, &bounds)
            .wall_vertex(v1, high, s1, t2, light_info, scroll, &bounds)
            .wall_quad();
    }

    fn visit_floor_poly(&mut self, poly: &StaticPoly) {
        self.num_floor_polys += 1;
        let &StaticPoly {
            vertices,
            height,
            light_info,
            tex_name,
        } = poly;
        let bounds = if let Some(bounds) = self.materials.flats.bounds.get(tex_name) {
            *bounds
        } else {
            warn!("No such floor texture {}.", tex_name);
            return;
        };
        let light_info = self.add_light_info(light_info);
        for vertex in vertices {
            self.flat_vertex(vertex, height, light_info, &bounds);
        }
        self.flat_poly(vertices.len());
    }

    fn visit_ceil_poly(&mut self, poly: &StaticPoly) {
        self.num_ceil_polys += 1;
        let &StaticPoly {
            vertices,
            height,
            light_info,
            tex_name,
        } = poly;
        let bounds = if let Some(bounds) = self.materials.flats.bounds.get(tex_name) {
            *bounds
        } else {
            warn!("No such ceiling texture {}.", tex_name);
            return;
        };
        let light_info = self.add_light_info(light_info);
        for vertex in vertices.iter().rev() {
            self.flat_vertex(vertex, height, light_info, &bounds);
        }
        self.flat_poly(vertices.len());
    }

    fn visit_floor_sky_poly(&mut self, &SkyPoly { vertices, height }: &SkyPoly) {
        self.num_sky_floor_polys += 1;
        for vertex in vertices {
            self.sky_vertex(vertex, height);
        }
        self.sky_poly(vertices.len());
    }

    fn visit_ceil_sky_poly(&mut self, &SkyPoly { vertices, height }: &SkyPoly) {
        self.num_sky_ceil_polys += 1;
        for vertex in vertices.iter().rev() {
            self.sky_vertex(vertex, height);
        }
        self.sky_poly(vertices.len());
    }

    fn visit_sky_quad(&mut self, quad: &SkyQuad) {
        self.num_sky_wall_quads += 1;
        let &SkyQuad {
            vertices: &(ref v1, ref v2),
            height_range: (low, high),
        } = quad;
        self.sky_vertex(v1, low)
            .sky_vertex(v2, low)
            .sky_vertex(v2, high)
            .sky_vertex(v1, high)
            .sky_quad();
    }

    fn visit_marker(&mut self, pos: Vec3f, yaw: f32, marker: Marker) {
        if let Marker::StartPos { player: 0 } = marker {
            self.start_pos = pos + Vec3f::new(0.0, 0.5, 32.0 / 100.0);
            self.start_yaw = yaw;
        }
    }

    fn visit_decor(&mut self, decor: &Decor) {
        self.num_decors += 1;
        let &Decor {
            low,
            high,
            half_width,
            light_info,
            tex_name,
        } = decor;
        let light_info = self.add_light_info(light_info);
        let bounds = if let Some(bounds) = self.materials.decor.bounds.get(tex_name) {
            *bounds
        } else {
            warn!("No such decor texture {}.", tex_name);
            return;
        };
        self.decor_vertex(low, -half_width, 0.0, bounds.size[1], &bounds, light_info)
            .decor_vertex(
                low,
                half_width,
                bounds.size[0],
                bounds.size[1],
                &bounds,
                light_info,
            )
            .decor_vertex(high, half_width, bounds.size[0], 0.0, &bounds, light_info)
            .decor_vertex(high, -half_width, 0.0, 0.0, &bounds, light_info)
            .decor_quad();
    }
}
