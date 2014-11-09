use gl;
use libc::c_void;
use math::{Mat4, Line2, Line2f, Vec2f, Vec2, Vec3f, Vec3, Numvec};
use gfx::{BufferBuilder, Renderer, RenderStep, ShaderLoader, VertexBuffer};
use std::rc::Rc;
use std::vec::Vec;
use wad;
use wad::SkyMetadata;
use wad::tex::{Bounds, BoundsLookup, TextureDirectory};
use wad::types::*;
use wad::util::{from_wad_height, from_wad_coords, is_untextured, parse_child_id,
                is_sky_flat};


pub struct Level {
    start_pos: Vec2f,
    renderer: Renderer,
}
impl Level {
    pub fn new(shader_loader: &ShaderLoader,
               wad: &mut wad::Archive,
               textures: &TextureDirectory,
               level_index: uint) -> Level {
        let (renderer, start_pos) = build_level(shader_loader,
                                                wad, textures, level_index);
        Level {
            renderer: renderer,
            start_pos: start_pos,
        }
    }

    pub fn get_start_pos(&self) -> &Vec2f { &self.start_pos }

    pub fn render(&mut self, delta_time: f32, projection_view: &Mat4) {
        self.renderer.render(delta_time, projection_view);
    }
}


struct TextureMaps {
    flats: BoundsLookup,
    walls: BoundsLookup,
}


struct RenderSteps { sky: RenderStep, flats: RenderStep,
    walls: RenderStep,
}


enum PegType {
    PegTop,
    PegBottom,
    PegBottomLower,
    PegTopFloat,
    PegBottomFloat
}

#[repr(packed)]
struct FlatVertex {
    _pos: Vec3f,
    _atlas_uv: Vec2f,
    _num_frames: u8,
    _frame_offset: u8,
    _light: u16,
}


#[repr(packed)]
struct WallVertex {
    _pos: Vec3f,
    _tile_uv: Vec2f,
    _atlas_uv: Vec2f,
    _tile_width: f32,
    _scroll_rate: f32,
    _num_frames: u8,
    _frame_offset: u8,
    _light: u16,
}


#[repr(packed)]
struct SkyVertex {
    _pos: Vec3f,
}



// Distance on the wrong side of a BSP and seg line allowed.
const BSP_TOLERANCE : f32 = 1e-3;
const SEG_TOLERANCE : f32 = 0.1;

// All polygons are `fattened' by this amount to fill in thin gaps between them.
const POLY_BIAS : f32 = 0.64 * 3e-4;

const PALETTE_UNIT: uint = 0;
const ATLAS_UNIT: uint = 1;


macro_rules! offset_of(
    ($T:ty, $m:ident) =>
        (unsafe { (&((*(0 as *const $T)).$m)) as *const _ as *const c_void })
)


pub fn build_level(shader_loader: &ShaderLoader,
                   wad: &mut wad::Archive,
                   textures: &wad::TextureDirectory,
                   level_index: uint)
        -> (Renderer, Vec2f) {
    let name = *wad.get_level_name(level_index);
    info!("Building level {}...", name);
    let level = wad::Level::from_archive(wad, level_index);

    let mut steps = RenderSteps {
        sky: init_sky_step(shader_loader, wad.get_metadata().sky_for(&name),
                           textures),
        flats: init_flats_step(shader_loader),
        walls: init_walls_step(shader_loader),
    };
    build_palette(textures, &mut [&mut steps.flats,
                                  &mut steps.sky,
                                  &mut steps.walls]);

    let texture_maps = TextureMaps {
        flats: build_flats_atlas(&level, textures, &mut steps.flats),
        walls: build_walls_atlas(&level, textures, &mut steps.walls),
    };

    VboBuilder::build(&level, &texture_maps, &mut steps);

    let mut renderer = Renderer::new();
    renderer
        .add_step(steps.sky)
        .add_step(steps.flats)
        .add_step(steps.walls);

    let mut start_pos = Vec2::zero();
    for thing in level.things.iter() {
        if thing.thing_type == 1 {  // Player 1 start position.
            start_pos = from_wad_coords(thing.x, thing.y);
            info!("Player start position: {}.", start_pos);
        }
    }
    (renderer, start_pos)
}


fn build_palette(textures: &TextureDirectory, steps: &mut [&mut RenderStep]) {
    let palette = Rc::new(textures.build_palette_texture(0, 0, 32));
    for step in steps.iter_mut() {
        step.add_shared_texture("u_palette", palette.clone(), PALETTE_UNIT);
    }
}

fn init_sky_step(shader_loader: &ShaderLoader,
                 meta: &wad::SkyMetadata, textures: &wad::TextureDirectory)
        -> RenderStep {
    let mut step = RenderStep::new(shader_loader.load("sky").unwrap());
    step.add_constant_f32("u_tiled_band_size", meta.tiled_band_size)
        .add_unique_texture("u_texture",
                            textures
                                .get_texture(&meta.texture_name)
                                .expect("init_sky_step: Missing sky texture.")
                                .to_texture(),
                            ATLAS_UNIT);
    step
}


fn init_flats_step(shader_loader: &ShaderLoader) -> RenderStep {
    RenderStep::new(shader_loader.load("flat").unwrap())
}


fn build_flats_atlas(level: &wad::Level, textures: &wad::TextureDirectory,
                     step: &mut RenderStep) -> BoundsLookup {
    struct SectorTexIter<'a> { sector: &'a WadSector, tex_index: uint }
    impl<'a> SectorTexIter<'a> {
        fn new(sector: &'a WadSector) -> SectorTexIter {
            SectorTexIter { sector: sector, tex_index: 0 }
        }
    }
    impl<'a> Iterator<&'a WadName> for SectorTexIter<'a> {
        fn next(&mut self) -> Option<&'a WadName> {
            self.tex_index += 1;
            match self.tex_index {
                1 => Some(&self.sector.floor_texture),
                2 => Some(&self.sector.ceiling_texture),
                _ => None
            }
        }
    }
    let flat_name_iter = level.sectors
            .iter()
            .flat_map(|s| SectorTexIter::new(s))
            .filter(|name| !is_untextured(*name) && !is_sky_flat(*name));
    let (atlas, lookup) = textures.build_flat_atlas(flat_name_iter);
    step.add_constant_vec2f("u_atlas_size", &atlas.size_as_vec())
        .add_unique_texture("u_atlas", atlas, ATLAS_UNIT);
    lookup
}


fn init_walls_step(shader_loader: &ShaderLoader) -> RenderStep {
    RenderStep::new(shader_loader.load("wall").unwrap())
}


fn build_walls_atlas(level: &wad::Level, textures: &wad::TextureDirectory,
                     step: &mut RenderStep) -> BoundsLookup {
    struct SidedefTexIter<'a> { side: &'a WadSidedef, tex_index: uint }
    impl<'a> SidedefTexIter<'a> {
        fn new(side: &'a WadSidedef) -> SidedefTexIter {
            SidedefTexIter { side: side, tex_index: 0 }
        }
    }
    impl<'a> Iterator<&'a WadName> for SidedefTexIter<'a> {
        fn next(&mut self) -> Option<&'a WadName> {
            self.tex_index += 1;
            match self.tex_index {
                1 => Some(&self.side.upper_texture),
                2 => Some(&self.side.lower_texture),
                3 => Some(&self.side.middle_texture),
                _ => None
            }
        }
    }
    let tex_name_iter = level.sidedefs
            .iter()
            .flat_map(|s| SidedefTexIter::new(s))
            .filter(|name| !is_untextured(*name));
    let (atlas, lookup) = textures.build_texture_atlas(tex_name_iter);
    step.add_constant_vec2f("u_atlas_size", &atlas.size_as_vec())
        .add_unique_texture("u_atlas", atlas, ATLAS_UNIT);

    lookup
}

type LightInfo = u16;


struct VboBuilder<'a> {
    level: &'a wad::Level,
    bounds: &'a TextureMaps,
    sky: Vec<SkyVertex>,
    flats: Vec<FlatVertex>,
    walls: Vec<WallVertex>,
    min_height: i16,
    max_height: i16,
}
impl<'a> VboBuilder<'a> {
    fn build(level: &wad::Level, bounds: &TextureMaps,
             steps: &mut RenderSteps) {
        let (min_height, max_height) = level.sectors
            .iter()
            .map(|s| (s.floor_height, s.ceiling_height))
            .fold((32767, -32768),
                  |(min, max), (f, c)| (if f < min { f } else { min },
                                        if c > max { c } else { max }));

        let mut builder = VboBuilder {
            level: level,
            bounds: bounds,
            flats: Vec::with_capacity(level.subsectors.len() * 4),
            walls: Vec::with_capacity(level.segs.len() * 2 * 4),
            sky: Vec::with_capacity(32),
            min_height: min_height,
            max_height: max_height,
        };
        let root_id = (level.nodes.len() - 1) as ChildId;
        builder.node(&mut Vec::with_capacity(32), root_id);

        let mut vbo = VboBuilder::init_sky_buffer();
        vbo.set_data(gl::STATIC_DRAW, builder.sky[]);
        steps.sky.add_static_vbo(vbo);

        let mut vbo = VboBuilder::init_flats_buffer();
        vbo.set_data(gl::STATIC_DRAW, builder.flats[]);
        steps.flats.add_static_vbo(vbo);

        let mut vbo = VboBuilder::init_walls_buffer();
        vbo.set_data(gl::STATIC_DRAW, builder.walls[]);
        steps.walls.add_static_vbo(vbo);

    }

    fn init_sky_buffer() -> VertexBuffer {
        let buffer = BufferBuilder::<SkyVertex>::new(2)
            .attribute_vec3f(0, offset_of!(SkyVertex, _pos))
            .build();
        buffer
    }

    fn init_flats_buffer() -> VertexBuffer {
        let buffer = BufferBuilder::<FlatVertex>::new(4)
            .attribute_vec3f(0, offset_of!(FlatVertex, _pos))
            .attribute_vec2f(1, offset_of!(FlatVertex, _atlas_uv))
            .attribute_u8(2, offset_of!(FlatVertex, _num_frames))
            .attribute_u8(3, offset_of!(FlatVertex, _frame_offset))
            .attribute_u16(4, offset_of!(FlatVertex, _light))
            .build();
        buffer
    }

    fn init_walls_buffer() -> VertexBuffer {
        let buffer = BufferBuilder::<WallVertex>::new(8)
            .attribute_vec3f(0, offset_of!(WallVertex, _pos))
            .attribute_vec2f(1, offset_of!(WallVertex, _tile_uv))
            .attribute_vec2f(2, offset_of!(WallVertex, _atlas_uv))
            .attribute_f32(3, offset_of!(WallVertex, _tile_width))
            .attribute_f32(4, offset_of!(WallVertex, _scroll_rate))
            .attribute_u8(5, offset_of!(WallVertex, _num_frames))
            .attribute_u8(6, offset_of!(WallVertex, _frame_offset))
            .attribute_u16(7, offset_of!(WallVertex, _light))
            .build();
        buffer
    }

    fn node(&mut self, lines: &mut Vec<Line2f>, id: ChildId) {
        let (id, is_leaf) = parse_child_id(id);
        if is_leaf {
            self.subsector(lines[mut], id);
            return;
        }

        let node = &self.level.nodes[id];
        let partition = Line2::from_origin_and_displace(
            from_wad_coords(node.line_x, node.line_y),
            from_wad_coords(node.step_x, node.step_y));
        lines.push(partition);
        self.node(lines, node.left);
        lines.pop();

        lines.push(partition.inverted_halfspaces());
        self.node(lines, node.right);
        lines.pop();
    }

    fn subsector(&mut self, lines: &[Line2f], id: uint) {
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
            seg_lines.push(Line2::from_two_points(v1, v2));

            // Also push the wall segments.
            self.seg(seg);
        }

        // The convex polyon defined at the intersection of the partition lines,
        // intersected with the half-volumes of the segs form the 'implicit'
        // points.
        for i_line in range(0, lines.len() - 1) {
            for j_line in range(i_line + 1, lines.len()) {
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
            self.flat_poly(self.level.seg_sector(&segs[0]), points[]);
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
                               if unpeg_lower { PegBottom } else { PegTop });
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

        if is_sky_flat(&sector.ceiling_texture)
                && !is_sky_flat(&back_sector.ceiling_texture)
                && !is_untextured(&side.upper_texture) {
            self.sky_quad(seg, (ceil, max));
        }
        if is_sky_flat(&sector.floor_texture)
                && !is_sky_flat(&back_sector.floor_texture)
                && !is_untextured(&side.lower_texture) {
            self.sky_quad(seg, (min, floor));
        }

        let unpeg_upper = line.upper_unpegged();
        let back_floor = back_sector.floor_height;
        let back_ceil = back_sector.ceiling_height;
        let floor = if back_floor > floor {
            self.wall_quad(seg, (floor, back_floor), &side.lower_texture,
                           if unpeg_lower { PegBottomLower } else { PegTop });
            back_floor
        } else {
            floor
        };
        let ceil = if back_ceil < ceil {
            if !is_sky_flat(&back_sector.ceiling_texture) {
                self.wall_quad(seg, (back_ceil, ceil), &side.upper_texture,
                               if unpeg_upper { PegTop } else { PegBottom });
            }
            back_ceil
        } else {
            ceil
        };
        self.wall_quad(seg, (floor, ceil), &side.middle_texture,
                       if unpeg_lower { PegTopFloat } else { PegBottomFloat });

    }

    fn wall_quad(&mut self, seg: &WadSeg, (low, high): (WadCoord, WadCoord),
                 texture_name: &WadName, peg: PegType) {
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
            PegTopFloat => (from_wad_height(low + side.y_offset),
                            from_wad_height(low + bounds.size.y as i16 +
                                            side.y_offset)),
            PegBottomFloat => (from_wad_height(high + side.y_offset -
                                               bounds.size.y as i16),
                               from_wad_height(high + side.y_offset)),
            _ => (from_wad_height(low), from_wad_height(high))
        };

        let light_info = self.light_info(sector);
        let height = (high - low) * 100.0;
        let s1 = seg.offset as f32 + side.x_offset as f32;
        let s2 = s1 + (v2 - v1).norm() * 100.0;
        let (t1, t2) = match peg {
            PegTop => (height, 0.0),
            PegBottom => (bounds.size.y, bounds.size.y - height),
            PegBottomLower => {
                // As far as I can tell, this is a special case.
                let sector_height = (sector.ceiling_height -
                                     sector.floor_height) as f32;
                (bounds.size.y + sector_height,
                 bounds.size.y - height + sector_height)
            }
            PegTopFloat | PegBottomFloat => {
                (bounds.size.y, 0.0)
            }
        };
        let (t1, t2) = (t1 + side.y_offset as f32, t2 + side.y_offset as f32);

        let scroll = if line.special_type == 0x30 { 35.0 }
                     else { 0.0 };

        let (low, high) = (low - POLY_BIAS, high + POLY_BIAS);
        self.wall_vertex(&v1, low,  s1, t1, light_info, scroll, bounds);
        self.wall_vertex(&v2, low,  s2, t1, light_info, scroll, bounds);
        self.wall_vertex(&v1, high, s1, t2, light_info, scroll, bounds);

        self.wall_vertex(&v2, low,  s2, t1, light_info, scroll, bounds);
        self.wall_vertex(&v2, high, s2, t2, light_info, scroll, bounds);
        self.wall_vertex(&v1, high, s1, t2, light_info, scroll, bounds);
    }

    fn flat_poly(&mut self, sector: &WadSector, points: &[Vec2f]) {
        let light_info = self.light_info(sector);
        let floor_y = from_wad_height(sector.floor_height);
        let floor_tex = &sector.floor_texture;
        let ceil_y = from_wad_height(sector.ceiling_height);
        let ceil_tex = &sector.ceiling_texture;

        let v0 = points[0];
        if !is_sky_flat(floor_tex) {
            let floor_bounds = self.bounds.flats
                .get(floor_tex)
                .expect(format!("flat: No such floor {}.", floor_tex)[]);
            for i in range(1, points.len()) {
                let (v1, v2) = (points[i], points[(i + 1) % points.len()]);
                self.flat_vertex(&v0, floor_y, light_info, floor_bounds);
                self.flat_vertex(&v1, floor_y, light_info, floor_bounds);
                self.flat_vertex(&v2, floor_y, light_info, floor_bounds);
            }
        } else {
            let min = from_wad_height(self.min_height);
            for i in range(1, points.len()) {
                let (v1, v2) = (points[i], points[(i + 1) % points.len()]);

                self.sky_vertex(&v0, min);
                self.sky_vertex(&v1, min);
                self.sky_vertex(&v2, min);
            }
        }

        if !is_sky_flat(ceil_tex) {
            let ceiling_bounds = self.bounds.flats
                .get(ceil_tex)
                .expect(format!("flat: No such ceiling {}.", ceil_tex)[]);
            for i in range(1, points.len()) {
                let (v1, v2) = (points[i], points[(i + 1) % points.len()]);
                self.flat_vertex(&v2, ceil_y, light_info, ceiling_bounds);
                self.flat_vertex(&v1, ceil_y, light_info, ceiling_bounds);
                self.flat_vertex(&v0, ceil_y, light_info, ceiling_bounds);
            }
        } else {
            let max = from_wad_height(self.max_height);
            for i in range(1, points.len()) {
                let (v1, v2) = (points[i], points[(i + 1) % points.len()]);

                self.sky_vertex(&v2, max);
                self.sky_vertex(&v1, max);
                self.sky_vertex(&v0, max);
            }
        }
    }

    fn sky_quad(&mut self, seg: &WadSeg, (low, high): (WadCoord, WadCoord)) {
        if low >= high { return; }
        let (v1, v2) = self.level.seg_vertices(seg);
        let bias = (v2 - v1).normalized() * POLY_BIAS;
        let (v1, v2) = (v1 - bias, v2 + bias);
        let (low, high) = (from_wad_height(low), from_wad_height(high));

        self.sky_vertex(&v1, low);
        self.sky_vertex(&v2, low);
        self.sky_vertex(&v1, high);

        self.sky_vertex(&v2, low);
        self.sky_vertex(&v2, high);
        self.sky_vertex(&v1, high);
    }


    fn light_info(&self, sector: &WadSector) -> u16 {
        let light = sector.light;
        let sector_id = self.level.sector_id(sector);
        let sync : u16 = (sector_id as uint * 1664525 + 1013904223) as u16;
        let min_light_or = |if_same| {
            let min = self.level.sector_min_light(sector);
            if min == light { if_same } else { min }
        };
        let (alt_light, light_type, sync) = match sector.sector_type {
            1   => (min_light_or(0),     0, sync), // FLASH
            2|4 => (min_light_or(0),     3, sync), // FAST STROBE
            3   => (min_light_or(0),     1, sync), // SLOW STROBE
            8   => (min_light_or(light), 4, 0),    // GLOW
            12  => (min_light_or(0),     1, 0),    // SLOW STROBE SYNC
            13  => (min_light_or(0),     3, 0),    // FAST STROBE SYNC
            17  => (min_light_or(0),     2, sync), // FLICKER
            _   => (light, 0, 0),
        };
        (((light as u16 >> 3) & 31) << 11) +
        (((alt_light as u16 >> 3) & 31) << 6) +
        (light_type << 3) +
        (sync & 7)
    }

    fn sky_vertex(&mut self, xz: &Vec2f, y: f32) {
        self.sky.push(SkyVertex {
            _pos: Vec3::new(xz.x, y, xz.y),
        });
    }

    fn flat_vertex(&mut self, xz: &Vec2f, y: f32, light_info: u16,
                   bounds: &Bounds) {
        self.flats.push(FlatVertex {
            _pos: Vec3::new(xz.x, y, xz.y),
            _atlas_uv: bounds.pos,
            _num_frames: bounds.num_frames as u8,
            _frame_offset: bounds.frame_offset as u8,
            _light: light_info,
        });
    }

    fn wall_vertex(&mut self, xz: &Vec2f, y: f32, tile_u: f32, tile_v: f32,
                   light_info: u16, scroll_rate: f32, bounds: &Bounds) {
        self.walls.push(WallVertex {
            _pos: Vec3::new(xz.x, y, xz.y),
            _tile_uv: Vec2::new(tile_u, tile_v),
            _atlas_uv: bounds.pos,
            _tile_width: bounds.size.x,
            _scroll_rate: scroll_rate,
            _num_frames: bounds.num_frames as u8,
            _frame_offset: bounds.frame_offset as u8,
            _light: light_info,
        });
    }
}

fn polygon_center(points: &[Vec2f]) -> Vec2f {
    let mut center = Vec2::zero();
    for p in points.iter() { center = center + *p; }
    center / (points.len() as f32)
}


fn points_to_polygon(points: &mut Vec<Vec2f>) {
    // Sort points in polygonal CCW order around their center.
    let center = polygon_center(points[mut]);
    points.sort_by(
        |a, b| {
            let ac = *a - center;
            let bc = *b - center;
            if ac.x >= 0.0 && bc.x < 0.0 {
                return Less;
            }
            if ac.x < 0.0 && bc.x >= 0.0 {
                return Greater;
            }
            if ac.x == 0.0 && bc.x == 0.0 {
                if ac.y >= 0.0 || bc.y >= 0.0 {
                    return if a.y > b.y { Less } else { Greater }
                }
                return if b.y > a.y { Less } else { Greater }
            }

            if ac.cross(&bc) < 0.0 { Less }
            else { Greater }
        });

    // Remove duplicates.
    let mut simplified = Vec::new();
    simplified.push((*points)[0]);
    let mut current_point = (*points)[1];
    let mut area = 0.0;
    for i_point in range(2, points.len()) {
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

    let center = polygon_center(simplified[]);
    for point in simplified.iter_mut() {
        *point = *point + (*point - center).normalized() * POLY_BIAS;
    }
    *points = simplified;
}
