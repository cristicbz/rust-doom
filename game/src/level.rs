use super::game_shaders::{GameShaders, LevelMaterials};
use super::lights::Lights;
use super::vertex::{SkyVertex, SpriteVertex, StaticVertex};
use super::wad_system::WadSystem;
use super::world::{World, WorldBuilder};
use engine::{
    DependenciesFrom, Entities, EntityId, Error, Meshes, RenderPipeline, Result, System, Tick,
    Transforms, Uniforms, Window,
};
use log::{debug, error, info, warn};
use math::prelude::*;
use math::{vec2, Line2f, Pnt2f, Pnt3f, Rad, Trans3, Vec3f};
use std::time::Instant;
use vec_map::VecMap;
use wad::tex::Bounds as WadBounds;
use wad::{
    Decor, LevelVisitor, LightInfo, Marker, MoveEffect, ObjectId, SkyPoly, SkyQuad, StaticPoly,
    StaticQuad, Trigger, TriggerType,
};

pub struct Level {
    root: EntityId,
    objects: Vec<EntityId>,
    triggers: Vec<Trigger>,
    removed: Vec<usize>,
    effects: VecMap<MoveEffect>,
    exit_triggered: bool,
    level_changed: bool,

    start_pos: Pnt3f,
    start_yaw: Rad<f32>,
    lights: Lights,
    volume: World,
}

#[derive(DependenciesFrom)]
pub struct Dependencies<'context> {
    window: &'context Window,
    entities: &'context mut Entities,
    uniforms: &'context mut Uniforms,
    meshes: &'context mut Meshes,
    render: &'context mut RenderPipeline,
    wad: &'context mut WadSystem,
    tick: &'context Tick,
    transforms: &'context mut Transforms,

    game_shaders: &'context GameShaders,
}

#[derive(Copy, Clone, Debug)]
pub enum PlayerAction {
    Push,
    Shoot,
}

impl Level {
    pub fn level_changed(&self) -> bool {
        self.level_changed
    }

    pub fn root(&self) -> EntityId {
        self.root
    }

    pub fn start_pos(&self) -> &Pnt3f {
        &self.start_pos
    }

    pub fn start_yaw(&self) -> Rad<f32> {
        self.start_yaw
    }

    pub fn volume(&self) -> &World {
        &self.volume
    }

    pub fn poll_triggers(
        &mut self,
        transform: &Trans3,
        moved: Vec3f,
        action: Option<PlayerAction>,
    ) {
        let position = Pnt2f::new(transform.disp.x, transform.disp.z);
        let walked = Line2f::from_origin_and_displace(position, vec2(-moved.x, -moved.z));
        let action_and_line = action.map(|action| {
            let look3d = transform.rot.rotate_vector(-Vec3f::unit_z());
            let look2d = vec2(look3d.x, look3d.z).normalize_or_zero();
            let ranged = look2d
                * match action {
                    PlayerAction::Push => 0.5,
                    PlayerAction::Shoot => 100.0,
                };
            (action, Line2f::from_origin_and_displace(position, ranged))
        });

        for (i_trigger, trigger) in self.triggers.iter().enumerate() {
            let mut triggered = false;
            match trigger.trigger_type {
                TriggerType::WalkOver => {
                    if let Some(offset) = walked.segment_intersect_offset(&trigger.line) {
                        debug!("Trigger {} walk-activated offset={}", i_trigger, offset);
                        triggered = true;
                    }
                }
                TriggerType::Push | TriggerType::Switch => {
                    if let Some((PlayerAction::Push, line)) = action_and_line {
                        if let Some(offset) = line.segment_intersect_offset(&trigger.line) {
                            debug!("Trigger {} push-activated offset={}", i_trigger, offset);
                            triggered = true;
                        }
                    }
                }
                TriggerType::Gun => {
                    if let Some((PlayerAction::Shoot, line)) = action_and_line {
                        if let Some(offset) = line.segment_intersect_offset(&trigger.line) {
                            debug!("Trigger {} shoot-activated offset={}", i_trigger, offset);
                            triggered = true;
                        }
                    }
                }
                TriggerType::Any => {
                    if let Some(offset) = walked.segment_intersect_offset(&trigger.line) {
                        debug!(
                            "Trigger {} (any) walk-activated offset={}",
                            i_trigger, offset
                        );
                        triggered = true;
                    } else if let Some((PlayerAction::Push, line)) = action_and_line {
                        if let Some(offset) = line.segment_intersect_offset(&trigger.line) {
                            debug!(
                                "Trigger {} (any) push-activated offset={}",
                                i_trigger, offset
                            );
                            triggered = true;
                        }
                    }
                }
            };
            if triggered {
                for &effect in &trigger.move_effects {
                    let effect_index = effect.object_id.0 as usize;
                    debug!(
                        "Started effect {} with type {}.",
                        effect_index, trigger.special_type
                    );
                    self.effects.insert(effect_index, effect);
                }

                if trigger.unimplemented {
                    error!("Unimpemented trigger: {}", trigger.special_type);
                }

                if trigger.only_once {
                    self.removed.push(i_trigger);
                }

                if trigger.exit_effect.is_some() {
                    self.exit_triggered = true;
                }
            }
        }

        for &i_removed in self.removed.iter().rev() {
            self.triggers.swap_remove(i_removed);
        }
        self.removed.clear()
    }
}

impl<'context> System<'context> for Level {
    type Dependencies = Dependencies<'context>;
    type Error = Error;

    fn debug_name() -> &'static str {
        "level"
    }

    fn create(mut deps: Dependencies) -> Result<Self> {
        Builder::build(&mut deps)
    }

    // Allow float_cmp, because we're checking equality against floats we set to a specific value.
    #[cfg_attr(feature = "cargo-clippy", allow(clippy::float_cmp))]
    fn update(&mut self, mut deps: Dependencies) -> Result<()> {
        if deps.wad.level_changed() {
            deps.entities.remove(self.root);
            *self = Builder::build(&mut deps)?;
            self.level_changed = true;
        } else if self.level_changed {
            info!("Level changed. {}", deps.entities.debug_tree_dump(4));
            self.level_changed = false;
        }

        if self.exit_triggered {
            self.exit_triggered = false;
            deps.entities.remove(self.root);
            let current_index = deps.wad.level_index();
            deps.wad.change_level(current_index + 1);
        }

        self.volume.update(deps.transforms);
        let timestep = deps.tick.timestep();
        for (i_effect, effect) in &mut self.effects {
            let entity_id = self.objects[i_effect];
            let transform = deps
                .transforms
                .get_local_mut(entity_id)
                .expect("no transform on object");
            let current_offset = &mut transform.disp[1];
            let mut timestep = timestep;

            loop {
                if effect.first_height_offset != *current_offset {
                    let offset_difference = effect.first_height_offset - *current_offset;
                    let sign = offset_difference.signum();
                    let time_left = offset_difference.abs() / effect.speed;
                    if time_left > timestep {
                        *current_offset += sign * effect.speed * timestep;
                        break;
                    } else {
                        *current_offset = effect.first_height_offset;
                        timestep -= time_left;
                        effect.first_height_offset = *current_offset;
                        debug!("Effect {}: finished first offset.", i_effect);
                    }
                }

                if effect.wait > timestep {
                    effect.wait -= timestep;
                    break;
                } else {
                    debug!("Effect {}: finished waiting.", i_effect);
                    timestep -= effect.wait;
                    effect.wait = 0.0;
                }

                if let Some(offset) = effect.second_height_offset.take() {
                    effect.first_height_offset = offset;
                    debug!(
                        "Effect {}: moved second offset {} into first.",
                        i_effect, offset
                    );
                    continue;
                }

                debug!("Effect {}: done, removing.", i_effect);
                self.removed.push(i_effect);
                break;
            }
        }

        for &i_removed in &self.removed {
            self.effects.remove(i_removed);
        }
        self.removed.clear();

        let time = *deps
            .uniforms
            .get_float_mut(deps.game_shaders.time())
            .expect("missing time");
        let light_infos = &mut self.lights;
        deps.uniforms
            .map_buffer_texture_u8(deps.game_shaders.lights_buffer_texture(), |buffer| {
                light_infos.fill_buffer_at(time, buffer)
            });
        Ok(())
    }

    fn teardown(&mut self, deps: Dependencies) -> Result<()> {
        deps.entities.remove(self.root);
        Ok(())
    }
}

struct Indices {
    wall: Vec<u32>,
    flat: Vec<u32>,
    sky: Vec<u32>,
    decor: Vec<u32>,
}

impl Indices {
    fn for_id(id: ObjectId) -> Self {
        if id == ObjectId(0) {
            Self::with_capacity(65_536)
        } else {
            Self::with_capacity(512)
        }
    }

    fn with_capacity(capacity: usize) -> Self {
        Indices {
            wall: Vec::with_capacity(capacity),
            flat: Vec::with_capacity(capacity),
            sky: Vec::with_capacity(capacity),
            decor: Vec::with_capacity(capacity),
        }
    }

    fn in_map(indices: &mut VecMap<Self>, object_id: ObjectId) -> &mut Self {
        indices
            .entry(object_id.0 as usize)
            .or_insert_with(|| Self::for_id(object_id))
    }
}

struct Builder<'a> {
    materials: &'a LevelMaterials,

    lights: Lights,
    start_pos: Pnt3f,
    start_yaw: Rad<f32>,

    static_vertices: Vec<StaticVertex>,
    sky_vertices: Vec<SkyVertex>,
    decor_vertices: Vec<SpriteVertex>,

    object_indices: VecMap<Indices>,

    num_wall_quads: usize,
    num_floor_polys: usize,
    num_ceil_polys: usize,
    num_sky_wall_quads: usize,
    num_sky_floor_polys: usize,
    num_sky_ceil_polys: usize,
    num_decors: usize,
}

impl<'a> Builder<'a> {
    fn build(deps: &mut Dependencies) -> Result<Level> {
        info!("Building new level...");

        let start_time = Instant::now();
        let root = deps.entities.add_root("level_root");

        let mut objects = Vec::new();
        let world = deps.entities.add(root, "world")?;
        deps.transforms.attach_identity(world);
        objects.extend((0..deps.wad.analysis.num_objects()).map(|i_object| {
            let entity = deps
                .entities
                .add(
                    world,
                    if i_object == 0 {
                        "static_object"
                    } else {
                        "dynamic_object"
                    },
                )
                .expect("add entity to world");
            deps.transforms.attach_identity(entity);
            entity
        }));

        let mut builder = Builder {
            materials: deps.game_shaders.level_materials(),

            lights: Lights::new(),
            start_pos: Pnt3f::origin(),
            start_yaw: Rad(0.0f32),

            static_vertices: Vec::with_capacity(16_384),
            sky_vertices: Vec::with_capacity(16_384),
            decor_vertices: Vec::with_capacity(16_384),

            object_indices: VecMap::new(),

            num_wall_quads: 0,
            num_floor_polys: 0,
            num_ceil_polys: 0,
            num_sky_wall_quads: 0,
            num_sky_floor_polys: 0,
            num_sky_ceil_polys: 0,
            num_decors: 0,
        };

        info!("Walking level...");
        let volume = {
            let mut world_builder = WorldBuilder::new(&objects);
            deps.wad.walk(&mut builder.chain(&mut world_builder));
            world_builder.build()
        };

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
            start_time.elapsed().f64_seconds() * 1000.0,
            builder.num_wall_quads,
            builder.num_floor_polys,
            builder.num_ceil_polys,
            builder.num_sky_wall_quads,
            builder.num_sky_floor_polys,
            builder.num_sky_ceil_polys,
            builder.num_decors,
            builder
                .object_indices
                .values()
                .map(|indices| indices.wall.len() + indices.flat.len())
                .sum::<usize>()
                / 3,
            builder
                .object_indices
                .values()
                .map(|indices| indices.sky.len())
                .sum::<usize>()
                / 3,
            builder
                .object_indices
                .values()
                .map(|indices| indices.decor.len())
                .sum::<usize>()
                / 3,
        );

        info!("Creating static meshes and models...");
        let global_static_mesh = deps
            .meshes
            .add(deps.window, deps.entities, root, "global_world_static_mesh")
            .immutable(&builder.static_vertices)?
            .build_unindexed()?;

        let global_sky_mesh = deps
            .meshes
            .add(deps.window, deps.entities, root, "global_world_sky_mesh")
            .immutable(&builder.sky_vertices)?
            .build_unindexed()?;

        let global_decor_mesh = deps
            .meshes
            .add(deps.window, deps.entities, root, "global_world_decor_mesh")
            .immutable(&builder.decor_vertices)?
            .build_unindexed()?;

        for (id, indices) in &builder.object_indices {
            let object = objects[id];
            if !indices.flat.is_empty() {
                let entity = deps.entities.add(object, "flats")?;
                let mesh = deps
                    .meshes
                    .add(deps.window, deps.entities, entity, "object_flats_mesh")
                    .shared(global_static_mesh)
                    .immutable_indices(&indices.flat)?
                    .build()?;
                deps.transforms.attach_identity(entity);
                deps.render
                    .attach_model(entity, mesh, builder.materials.flats.material);
            }

            if !indices.wall.is_empty() {
                let entity = deps.entities.add(object, "walls")?;
                let mesh = deps
                    .meshes
                    .add(deps.window, deps.entities, entity, "object_walls_mesh")
                    .shared(global_static_mesh)
                    .immutable_indices(&indices.wall)?
                    .build()?;
                deps.transforms.attach_identity(entity);
                deps.render
                    .attach_model(entity, mesh, builder.materials.walls.material);
            }

            if !indices.decor.is_empty() {
                let entity = deps.entities.add(object, "decor")?;
                let mesh = deps
                    .meshes
                    .add(deps.window, deps.entities, entity, "object_decor_mesh")
                    .shared(global_decor_mesh)
                    .immutable_indices(&indices.decor)?
                    .build()?;
                deps.transforms.attach_identity(entity);
                deps.render
                    .attach_model(entity, mesh, builder.materials.decor.material);
            }

            if !indices.sky.is_empty() {
                let entity = deps.entities.add(object, "sky")?;
                let mesh = deps
                    .meshes
                    .add(deps.window, deps.entities, entity, "object_sky_mesh")
                    .shared(global_sky_mesh)
                    .immutable_indices(&indices.sky)?
                    .build()?;
                deps.transforms.attach_identity(entity);
                deps.render
                    .attach_model(entity, mesh, builder.materials.sky);
            }
        }

        Ok(Level {
            root,
            volume,
            objects,
            triggers: deps.wad.analysis.take_triggers(),
            removed: Vec::with_capacity(128),
            effects: VecMap::new(),
            start_pos: builder.start_pos,
            start_yaw: builder.start_yaw,
            lights: builder.lights,
            exit_triggered: false,
            level_changed: true,
        })
    }

    #[cfg_attr(feature = "cargo-clippy", allow(clippy::too_many_arguments))]
    fn wall_vertex(
        &mut self,
        xz: Pnt2f,
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

    fn flat_vertex(&mut self, xz: Pnt2f, y: f32, light_info: u8, bounds: &WadBounds) -> &mut Self {
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

    fn sky_vertex(&mut self, xz: Pnt2f, y: f32) -> &mut Self {
        self.sky_vertices.push(SkyVertex {
            a_pos: [xz[0], y, xz[1]],
        });
        self
    }

    fn decor_vertex(
        &mut self,
        pos: Pnt3f,
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

    fn flat_poly(&mut self, object_id: ObjectId, poly_length: usize) {
        Self::any_poly(
            self.static_vertices.len(),
            poly_length,
            &mut Indices::in_map(&mut self.object_indices, object_id).flat,
        );
    }

    fn wall_quad(&mut self, object_id: ObjectId) {
        Self::any_quad(
            self.static_vertices.len(),
            &mut Indices::in_map(&mut self.object_indices, object_id).wall,
        );
    }

    fn sky_poly(&mut self, object_id: ObjectId, poly_length: usize) {
        Self::any_poly(
            self.sky_vertices.len(),
            poly_length,
            &mut Indices::in_map(&mut self.object_indices, object_id).sky,
        );
    }

    fn sky_quad(&mut self, object_id: ObjectId) {
        Self::any_quad(
            self.sky_vertices.len(),
            &mut Indices::in_map(&mut self.object_indices, object_id).sky,
        );
    }

    fn decor_quad(&mut self, object_id: ObjectId) {
        Self::any_quad(
            self.decor_vertices.len(),
            &mut Indices::in_map(&mut self.object_indices, object_id).decor,
        );
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

impl<'a> LevelVisitor for Builder<'a> {
    // TODO(cristicbz): Change some types here and unify as much as possible.
    fn visit_wall_quad(&mut self, quad: &StaticQuad) {
        self.num_wall_quads += 1;
        let &StaticQuad {
            object_id,
            tex_name,
            light_info,
            scroll,
            vertices: (v1, v2),
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
        let bounds = if let Some(bounds) = self.materials.walls.bounds.get(&tex_name) {
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
            .wall_quad(object_id);
    }

    fn visit_floor_poly(&mut self, poly: &StaticPoly) {
        self.num_floor_polys += 1;
        let &StaticPoly {
            object_id,
            vertices,
            height,
            light_info,
            tex_name,
        } = poly;
        let bounds = if let Some(bounds) = self.materials.flats.bounds.get(&tex_name) {
            *bounds
        } else {
            warn!("No such floor texture {}.", tex_name);
            return;
        };
        let light_info = self.add_light_info(light_info);
        for &vertex in vertices {
            self.flat_vertex(vertex, height, light_info, &bounds);
        }
        self.flat_poly(object_id, vertices.len());
    }

    fn visit_ceil_poly(&mut self, poly: &StaticPoly) {
        self.num_ceil_polys += 1;
        let &StaticPoly {
            object_id,
            vertices,
            height,
            light_info,
            tex_name,
        } = poly;
        let bounds = if let Some(bounds) = self.materials.flats.bounds.get(&tex_name) {
            *bounds
        } else {
            warn!("No such ceiling texture {}.", tex_name);
            return;
        };
        let light_info = self.add_light_info(light_info);
        for &vertex in vertices.iter().rev() {
            self.flat_vertex(vertex, height, light_info, &bounds);
        }
        self.flat_poly(object_id, vertices.len());
    }

    fn visit_floor_sky_poly(&mut self, poly: &SkyPoly) {
        self.num_sky_floor_polys += 1;
        for &vertex in poly.vertices {
            self.sky_vertex(vertex, poly.height);
        }
        self.sky_poly(poly.object_id, poly.vertices.len());
    }

    fn visit_ceil_sky_poly(&mut self, poly: &SkyPoly) {
        self.num_sky_ceil_polys += 1;
        for &vertex in poly.vertices.iter().rev() {
            self.sky_vertex(vertex, poly.height);
        }
        self.sky_poly(poly.object_id, poly.vertices.len());
    }

    fn visit_sky_quad(&mut self, quad: &SkyQuad) {
        self.num_sky_wall_quads += 1;
        let &SkyQuad {
            object_id,
            vertices: (v1, v2),
            height_range: (low, high),
        } = quad;
        self.sky_vertex(v1, low)
            .sky_vertex(v2, low)
            .sky_vertex(v2, high)
            .sky_vertex(v1, high)
            .sky_quad(object_id);
    }

    fn visit_marker(&mut self, pos: Pnt3f, yaw: Rad<f32>, marker: Marker) {
        if let Marker::StartPos { player: 0 } = marker {
            self.start_pos = pos + Vec3f::new(0.0, 0.5, 32.0 / 100.0);
            self.start_yaw = yaw;
        }
    }

    fn visit_decor(&mut self, decor: &Decor) {
        self.num_decors += 1;
        let &Decor {
            object_id,
            low,
            high,
            half_width,
            light_info,
            tex_name,
        } = decor;
        let light_info = self.add_light_info(light_info);
        let bounds = if let Some(bounds) = self.materials.decor.bounds.get(&tex_name) {
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
            .decor_quad(object_id);
    }
}
