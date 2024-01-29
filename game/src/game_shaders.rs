use crate::vertex::{SkyVertex, SpriteVertex, StaticVertex};

use super::wad_system::WadSystem;
use engine::{
    DependenciesFrom, Entities, EntityId, Error, FloatUniformId, MaterialId, Materials, Result,
    ShaderId, ShaderVertex, Shaders, System, Tick, Uniforms, Window, LIGHTS_COUNT,
};
use log::{error, info};
use math::Vec2;
use wad::tex::BoundsLookup;
use wad::types::{COLORMAP_SIZE, MAPPED_PALETTE_SIZE};
use wad::util::{is_sky_flat, is_untextured};
use wad::{OpaqueImage as WadOpaqueImage, TransparentImage as WadTransparentImage, WadName};

pub struct AtlasMaterial {
    pub material: MaterialId,
    pub bounds: BoundsLookup,
}

pub struct LevelMaterials {
    pub flats: AtlasMaterial,
    pub walls: AtlasMaterial,
    pub decor: AtlasMaterial,
    pub sky: MaterialId,
}

impl GameShaders {
    pub fn time(&self) -> FloatUniformId {
        self.globals.time
    }

    pub fn lights_buffer(&self) -> &wgpu::Buffer {
        &self.globals.lights_buffer
    }

    pub fn level_materials(&self) -> &LevelMaterials {
        &self.level
    }
}

#[derive(DependenciesFrom)]
pub struct Dependencies<'context> {
    tick: &'context Tick,
    window: &'context Window,
    entities: &'context mut Entities,
    shaders: &'context mut Shaders,
    uniforms: &'context mut Uniforms,
    materials: &'context mut Materials,

    wad: &'context mut WadSystem,
}

impl<'context> System<'context> for GameShaders {
    type Dependencies = Dependencies<'context>;
    type Error = Error;

    fn debug_name() -> &'static str {
        "game_shaders"
    }

    fn create(mut deps: Dependencies) -> Result<Self> {
        let globals_id = deps.entities.add_root("game_shaders");
        let level_id = deps.entities.add(globals_id, "level_materials")?;

        let globals = deps.load_globals(globals_id)?;
        let level = deps.load_level(&globals, level_id)?;

        Ok(GameShaders {
            globals_id,
            level_id,
            globals,
            level,
        })
    }

    fn update(&mut self, mut deps: Dependencies) -> Result<()> {
        if deps.wad.level_changed() {
            info!("Level changed, reloading level materials...");
            deps.entities.remove(self.level_id);
            self.level_id = deps.entities.add(self.globals_id, "level_materials")?;
            self.level = deps.load_level(&self.globals, self.level_id)?;
            info!("Reloaded level materials.");
            *deps
                .uniforms
                .get_float_mut(self.globals.time)
                .expect("missing time") = 0.0;
        } else {
            deps.uniforms
                .increment_time(deps.tick.timestep(), deps.window.queue());
        }

        Ok(())
    }

    fn teardown(&mut self, deps: Dependencies) -> Result<()> {
        deps.entities.remove(self.level_id);
        deps.entities.remove(self.globals_id);
        Ok(())
    }
}

pub struct GameShaders {
    globals_id: EntityId,
    level_id: EntityId,

    globals: Globals,
    level: LevelMaterials,
}

struct Globals {
    time: FloatUniformId,
    lights_buffer: wgpu::Buffer,
    static_shader: ShaderId,
    sky_shader: ShaderId,
    sprite_shader: ShaderId,
}

impl<'context> Dependencies<'context> {
    fn load_palette(&mut self, parent: EntityId) -> Result<wgpu::Texture> {
        let palette = self.wad.textures.build_palette_texture(0, 0, 32);
        self.uniforms.add_texture_2d(
            self.window,
            self.entities,
            parent,
            "palette",
            &palette.pixels,
            Vec2::new(COLORMAP_SIZE, palette.pixels.len() / MAPPED_PALETTE_SIZE),
            wgpu::TextureFormat::Rgba8UnormSrgb,
        )
    }

    fn load_globals(&mut self, parent: EntityId) -> Result<Globals> {
        let palette = self.load_palette(parent)?;

        let time = self
            .uniforms
            .add_float(self.entities, parent, "time_uniform", 0.0)?;
        let lights_buffer = self.uniforms.add_persistent_buffer(
            self.window,
            self.entities,
            parent,
            "lights_buffer_texture",
            LIGHTS_COUNT * std::mem::size_of::<u32>(),
        )?;

        let static_shader = self.load_shader::<StaticVertex>(
            parent,
            "static_shader",
            "static",
            include_str!("../../assets/shaders/static.wgsl"),
        )?;
        let sky_shader = self.load_shader::<SkyVertex>(
            parent,
            "sky_shader",
            "sky",
            include_str!("../../assets/shaders/sky.wgsl"),
        )?;
        let sprite_shader = self.load_shader::<SpriteVertex>(
            parent,
            "sprite_shader",
            "sprite",
            include_str!("../../assets/shaders/sprite.wgsl"),
        )?;

        self.uniforms
            .set_globals(self.window.device(), self.shaders, &lights_buffer, &palette);

        Ok(Globals {
            time,
            lights_buffer,
            static_shader,
            sky_shader,
            sprite_shader,
        })
    }

    fn load_level(&mut self, globals: &Globals, parent: EntityId) -> Result<LevelMaterials> {
        let flats_atlas = self.load_flats_atlas(parent)?;
        let flats_material = self
            .materials
            .add(
                self.entities,
                parent,
                globals.static_shader,
                "flats_material",
            )?
            .with_atlas(&flats_atlas.texture, self.window.device(), self.shaders)
            .id();

        let walls_atlas = self.load_walls_atlas(parent)?;
        let walls_material = self
            .materials
            .add(
                self.entities,
                parent,
                globals.static_shader,
                "walls_material",
            )?
            .with_atlas(&walls_atlas.texture, self.window.device(), self.shaders)
            .id();

        let sky_uniforms = self.load_sky_uniforms(parent)?;
        let sky_material = self
            .materials
            .add(self.entities, parent, globals.sky_shader, "sky_material")?
            .with_sky_texture(
                &sky_uniforms.texture,
                sky_uniforms.tiled_band_size,
                self.window.device(),
                self.shaders,
            )
            .id();

        let decor_atlas = self.load_decor_atlas(parent)?;
        let decor_material = self
            .materials
            .add(
                self.entities,
                parent,
                globals.sprite_shader,
                "decor_material",
            )?
            .with_atlas(&decor_atlas.texture, self.window.device(), self.shaders)
            .id();

        Ok(LevelMaterials {
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
        })
    }

    fn load_flats_atlas(&mut self, parent: EntityId) -> Result<Atlas> {
        info!("Building flats atlas...");
        let (image, bounds) = {
            let names = self
                .wad
                .level
                .sectors
                .iter()
                .flat_map(|sector| {
                    Some(sector.floor_texture)
                        .into_iter()
                        .chain(Some(sector.ceiling_texture))
                })
                .filter(|&name| !is_untextured(name) && !is_sky_flat(name));
            self.wad.textures.build_flat_atlas(names)
        };
        let texture = self.load_wad_texture(
            parent,
            "flats_atlas_texture",
            TextureSpec::OpaqueAtlas(&image),
        )?;
        Ok(Atlas { texture, bounds })
    }

    fn load_walls_atlas(&mut self, parent: EntityId) -> Result<Atlas> {
        info!("Building walls atlas...");
        let (image, bounds) = {
            let names = self
                .wad
                .level
                .sidedefs
                .iter()
                .flat_map(|sidedef| {
                    Some(sidedef.upper_texture)
                        .into_iter()
                        .chain(Some(sidedef.lower_texture))
                        .chain(Some(sidedef.middle_texture))
                })
                .filter(|&name| !is_untextured(name));
            self.wad.textures.build_texture_atlas(names)
        };
        let texture = self.load_wad_texture(
            parent,
            "walls_atlas_texture",
            TextureSpec::TransparentAtlas(&image),
        )?;
        Ok(Atlas { texture, bounds })
    }

    fn load_decor_atlas(&mut self, parent: EntityId) -> Result<Atlas> {
        info!("Building sprite decorations atlas...");
        let (image, bounds) = {
            let wad = &self.wad;
            let names = wad
                .level
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
            TextureSpec::TransparentAtlas(&image),
        )?;
        Ok(Atlas { texture, bounds })
    }

    fn load_sky_uniforms(&mut self, parent: EntityId) -> Result<SkyUniforms> {
        let (texture_name, tiled_band_size) = self
            .wad
            .archive
            .metadata()
            .sky_for(self.wad.level_name())
            .map_or_else(
                || {
                    error!("No sky texture for level, will not render skies.");
                    (
                        WadName::from_bytes(b"-").expect("cannot convert dummy name"),
                        0.0,
                    )
                },
                |meta| (meta.texture_name, meta.tiled_band_size),
            );
        Ok(SkyUniforms {
            texture: self.load_wad_texture(
                parent,
                "sky_texture",
                TextureSpec::TextureName(texture_name),
            )?,
            tiled_band_size,
        })
    }

    fn load_wad_texture(
        &mut self,
        parent: EntityId,
        name: &'static str,
        texture_spec: TextureSpec,
    ) -> Result<wgpu::Texture> {
        let dummy_texture;
        let image_ref = match texture_spec {
            TextureSpec::TextureName(texture_name) => {
                if let Some(image) = self.wad.textures.texture(texture_name) {
                    ImageRef::Transparent {
                        pixels: image.pixels(),
                        size: image.size(),
                    }
                } else {
                    error!("Missing texture {:?} for {:?}.", texture_name, name);
                    dummy_texture = [0u16];
                    ImageRef::Transparent {
                        pixels: &dummy_texture,
                        size: Vec2::new(1, 1),
                    }
                }
            }
            TextureSpec::TransparentAtlas(image) => ImageRef::Transparent {
                pixels: &image.pixels,
                size: image.size,
            },
            TextureSpec::OpaqueAtlas(image) => ImageRef::Opaque {
                pixels: &image.pixels,
                size: image.size,
            },
        };
        Ok(match image_ref {
            ImageRef::Transparent { pixels, size } => self.uniforms.add_texture_2d(
                self.window,
                self.entities,
                parent,
                name,
                pixels,
                size,
                wgpu::TextureFormat::Rg8Unorm,
            )?,
            ImageRef::Opaque { pixels, size } => self.uniforms.add_texture_2d(
                self.window,
                self.entities,
                parent,
                name,
                pixels,
                size,
                wgpu::TextureFormat::R8Unorm,
            )?,
        })
    }

    fn load_shader<VertexT: ShaderVertex>(
        &mut self,
        parent: EntityId,
        name: &'static str,
        asset: &'static str,
        wgsl_source: &'static str,
    ) -> Result<ShaderId> {
        self.shaders
            .add::<VertexT>(self.window, self.entities, parent, name, asset, wgsl_source)
    }
}

struct SkyUniforms {
    tiled_band_size: f32,
    texture: wgpu::Texture,
}

struct Atlas {
    texture: wgpu::Texture,
    bounds: BoundsLookup,
}

#[derive(Copy, Clone)]
enum TextureSpec<'a> {
    TransparentAtlas(&'a WadTransparentImage),
    OpaqueAtlas(&'a WadOpaqueImage),
    TextureName(WadName),
}

enum ImageRef<'a> {
    Transparent {
        pixels: &'a [u16],
        size: Vec2<usize>,
    },
    Opaque {
        pixels: &'a [u8],
        size: Vec2<usize>,
    },
}
