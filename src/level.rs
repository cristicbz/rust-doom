use gl;
use libc::c_void;
use line::{Line2, Line2f};
use mat4::Mat4;
use numvec::{Vec2f, Vec2, Vec3f, Vec3, Numvec};
use render::{Renderer, RenderStep};
use shader::Shader;
use std::collections::{HashSet, HashMap};
use std::hash::sip::SipHasher;
use std::rc::Rc;
use std::str;
use std::vec::Vec;
use vbo::{BufferBuilder, VertexBuffer};
use wad;
use wad::tex::{Bounds, TextureDirectory};
use wad::types::*;
use wad::util::{from_wad_height, from_wad_coords, is_untextured, parse_child_id,
                name_toupper, is_sky_flat};


pub struct Level {
    start_pos: Vec2f,
    renderer: Renderer,
}
impl Level {
    pub fn new(wad: &mut wad::Archive,
               textures: &TextureDirectory,
               name: &WadName) -> Level {
        let (renderer, start_pos) = build_level(wad, textures, name);
        Level {
            renderer: renderer,
            start_pos: start_pos,
        }
    }

    pub fn get_start_pos<'a>(&'a self) -> &'a Vec2f { &self.start_pos }

    pub fn render(&mut self, delta_time: f32, projection_view: &Mat4) {
        self.renderer.render(delta_time, projection_view);
    }
}


type BoundsLookup = HashMap<Vec<u8>, Bounds>;


struct TextureMaps {
    flats: BoundsLookup,
    walls: BoundsLookup,
}


struct RenderSteps {
    flats: RenderStep,
    walls: RenderStep,
}


enum PegType {
    PegTop,
    PegBottom,
    PegBottomLower,
}


#[repr(packed)]
struct FlatVertex {
    _pos: Vec3f,
    _offsets: Vec2f,
    _brightness: f32,
    _num_frames: u8,
    _frame_offset: u8,
}


#[repr(packed)]
struct WallVertex {
    _pos: Vec3f,
    _tile_uv: Vec2f,
    _atlas_uv: Vec2f,
    _tile_width: f32,
    _brightness: f32,
    _scroll_rate: f32,
    _num_frames: u8,
    _frame_offset: u8,
}


// Distance on the wrong side of a BSP and seg line allowed.
static BSP_TOLERANCE : f32 = 1e-3;
static SEG_TOLERANCE : f32 = 0.1;

// All polygons are `fattened' by this amount to fill in thin gaps between them.
static POLY_BIAS : f32 = 0.64 * 3e-4;

static PALETTE_UNIT: uint = 0;
static ATLAS_UNIT: uint = 1;


macro_rules! offset_of(
    ($T:ty, $m:ident) =>
        (unsafe { (&((*(0 as *const $T)).$m)) as *const _ as *const c_void })
)


pub fn build_level(wad: &mut wad::Archive,
                   textures: &wad::TextureDirectory,
                   level_name: &WadName)
        -> (Renderer, Vec2f) {
    info!("Building level {}...", str::from_utf8(level_name));
    let level = wad::Level::from_archive(wad, level_name);

    let mut steps = RenderSteps {
        flats: init_flats_step(),
        walls: init_walls_step(),
    };
    build_palette(textures, &mut [&mut steps.flats, &mut steps.walls]);

    let texture_maps = TextureMaps {
        flats: build_flats_atlas(&level, textures, &mut steps.flats),
        walls: build_walls_atlas(&level, textures, &mut steps.walls),
    };

    VboBuilder::build(&level, &texture_maps, &mut steps);

    let mut renderer = Renderer::new();
    renderer.add_step(steps.flats).add_step(steps.walls);

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
    let palette = Rc::new(textures.build_palette_texture(0, 0, 31));
    for step in steps.iter_mut() {
        step.add_shared_texture("u_palette", palette.clone(), PALETTE_UNIT);
    }
}


fn init_flats_step() -> RenderStep {
    RenderStep::new(Shader::new_from_files(
            &Path::new("src/shaders/flat.vertex.glsl"),
            &Path::new("src/shaders/flat.fragment.glsl")).unwrap())
}


fn build_flats_atlas(level: &wad::Level, textures: &wad::TextureDirectory,
                     step: &mut RenderStep) -> BoundsLookup {
    let mut flats = HashSet::with_hasher(SipHasher::new());
    for sector in level.sectors.iter() {
        flats.insert(name_toupper(sector.floor_texture));
        flats.insert(name_toupper(sector.ceiling_texture));
    }
    let (atlas, lookup) = textures.build_flat_atlas(
        flats.iter().map(|x| x.as_slice()));
    step.add_constant_vec2f("u_atlas_size", &atlas.size_as_vec())
        .add_unique_texture("u_atlas", atlas, ATLAS_UNIT);
    lookup
}


fn init_walls_step() -> RenderStep {
    RenderStep::new(Shader::new_from_files(
        &Path::new("src/shaders/wall.vertex.glsl"),
        &Path::new("src/shaders/wall.fragment.glsl")).unwrap())
}


fn build_walls_atlas(level: &wad::Level, textures: &wad::TextureDirectory,
                     step: &mut RenderStep) -> BoundsLookup {
    let mut walls = HashSet::with_hasher(SipHasher::new());
    for sidedef in level.sidedefs.iter() {
        if !is_untextured(&sidedef.upper_texture) {
            walls.insert(name_toupper(sidedef.upper_texture));
        }
        if !is_untextured(&sidedef.middle_texture) {
            walls.insert(name_toupper(sidedef.middle_texture));
        }
        if !is_untextured(&sidedef.lower_texture) {
            walls.insert(name_toupper(sidedef.lower_texture));
        }
    }
    let (atlas, lookup) = textures.build_picture_atlas(
        walls.iter().map(|x| x.as_slice()));
    step.add_constant_vec2f("u_atlas_size", &atlas.size_as_vec())
        .add_unique_texture("u_atlas", atlas, ATLAS_UNIT);

    lookup
}

struct VboBuilder<'a> {
    level: &'a wad::Level,
    bounds: &'a TextureMaps,
    flats: Vec<FlatVertex>,
    walls: Vec<WallVertex>,
}
impl<'a> VboBuilder<'a> {
    fn build(level: &'a wad::Level, bounds: &'a TextureMaps,
             steps: &mut RenderSteps) {
        let mut builder = VboBuilder {
            level: level,
            bounds: bounds,
            flats: Vec::with_capacity(level.subsectors.len() * 4),
            walls: Vec::with_capacity(level.segs.len() * 2 * 4),
        };
        let root_id = (level.nodes.len() - 1) as ChildId;
        builder.node(&mut Vec::with_capacity(32), root_id);

        let mut vbo = VboBuilder::create_flats_buffer();
        vbo.set_data(gl::STATIC_DRAW, builder.flats.as_slice());
        steps.flats.add_static_vbo(vbo);

        let mut vbo = VboBuilder::create_walls_buffer();
        vbo.set_data(gl::STATIC_DRAW, builder.walls.as_slice());
        steps.walls.add_static_vbo(vbo);
    }

    fn create_flats_buffer() -> VertexBuffer {
        let buffer = BufferBuilder::<FlatVertex>::new(4)
            .attribute_vec3f(0, offset_of!(FlatVertex, _pos))
            .attribute_vec2f(1, offset_of!(FlatVertex, _offsets))
            .attribute_f32(2, offset_of!(FlatVertex, _brightness))
            .attribute_u8(3, offset_of!(FlatVertex, _num_frames))
            .attribute_u8(4, offset_of!(FlatVertex, _frame_offset))
            .build();
        buffer
    }

    fn create_walls_buffer() -> VertexBuffer {
        let buffer = BufferBuilder::<WallVertex>::new(8)
            .attribute_vec3f(0, offset_of!(WallVertex, _pos))
            .attribute_vec2f(1, offset_of!(WallVertex, _tile_uv))
            .attribute_vec2f(2, offset_of!(WallVertex, _atlas_uv))
            .attribute_f32(3, offset_of!(WallVertex, _tile_width))
            .attribute_f32(4, offset_of!(WallVertex, _brightness))
            .attribute_f32(5, offset_of!(WallVertex, _scroll_rate))
            .attribute_u8(6, offset_of!(WallVertex, _num_frames))
            .attribute_u8(7, offset_of!(WallVertex, _frame_offset))
            .build();
        buffer
    }

    fn node(&mut self, lines: &mut Vec<Line2f>, id: ChildId) {
        let (id, is_leaf) = parse_child_id(id);
        if is_leaf {
            self.subsector(lines.as_slice(), id);
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
        // intersected with the half-volume of the segs form the 'implicit'
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

        points_to_polygon(&mut points);  // Sort and remove duplicates.
        self.flat_poly(self.level.seg_sector(&segs[0]), points.as_slice());
    }

    fn seg(&mut self, seg: &WadSeg) {
        let line = self.level.seg_linedef(seg);
        let side = self.level.seg_sidedef(seg);
        let sector = self.level.sidedef_sector(side);
        let (floor, ceil) = (sector.floor_height, sector.ceiling_height);
        let unpeg_lower = line.lower_unpegged();
        let back_sector = match self.level.seg_back_sector(seg) {
            None => {
                self.wall_quad(seg, (floor, ceil), &side.middle_texture,
                               if unpeg_lower { PegBottom } else { PegTop });
                return
            },
            Some(s) => s
        };

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
                       if unpeg_lower { PegBottom } else { PegTop });

    }

    fn wall_quad(&mut self, seg: &WadSeg, (low, high): (WadCoord, WadCoord),
                 texture_name: &[u8, ..8], peg: PegType) {
        if is_untextured(texture_name) { return; }
        let bounds = match self.bounds.walls.find(&name_toupper(texture_name)) {
            None => {
                fail!("wall_quad: No such wall texture '{}'",
                      str::from_utf8(texture_name));
            },
            Some(bounds) => bounds,
        };

        let line = self.level.seg_linedef(seg);
        let side = self.level.seg_sidedef(seg);
        let sector = self.level.sidedef_sector(side);
        let (v1, v2) = self.level.seg_vertices(seg);
        let bias = (v2 - v1).normalized() * POLY_BIAS;
        let (v1, v2) = (v1 - bias, v2 + bias);
        let (low, high) = (from_wad_height(low), from_wad_height(high));

        let brightness = sector.light as f32 / 255.0;
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
        };
        let (t1, t2) = (t1 + side.y_offset as f32, t2 + side.y_offset as f32);

        let scroll = if line.special_type == 0x30 { 35.0 }
                     else { 0.0 };

        let (low, high) = (low - POLY_BIAS, high + POLY_BIAS);
        self.wall_vertex(&v1, low,  s1, t1, brightness, scroll, bounds);
        self.wall_vertex(&v2, low,  s2, t1, brightness, scroll, bounds);
        self.wall_vertex(&v1, high, s1, t2, brightness, scroll, bounds);

        self.wall_vertex(&v2, low,  s2, t1, brightness, scroll, bounds);
        self.wall_vertex(&v2, high, s2, t2, brightness, scroll, bounds);
        self.wall_vertex(&v1, high, s1, t2, brightness, scroll, bounds);
    }

    fn flat_poly(&mut self, sector: &WadSector, points: &[Vec2f]) {
        let floor = from_wad_height(sector.floor_height);
        let ceiling = from_wad_height(sector.ceiling_height);
        let floor_flat = name_toupper(sector.floor_texture);
        let ceiling_flat = name_toupper(sector.ceiling_texture);
        let floor_bounds = self.bounds.flats
            .find(&floor_flat)
            .expect("flat_poly: No such floor texture.");
        let ceiling_bounds = self.bounds.flats
            .find(&ceiling_flat)
            .expect("flat_poly: No such ceiling texture.");
        let bright = sector.light as f32 / 255.0;
        let v0 = points[0];

        if !is_sky_flat(&sector.floor_texture) {
            for i in range(1, points.len()) {
                let (v1, v2) = (points[i], points[(i + 1) % points.len()]);
                self.flat_vertex(&v0, floor, bright, floor_bounds);
                self.flat_vertex(&v1, floor, bright, floor_bounds);
                self.flat_vertex(&v2, floor, bright, floor_bounds);
            }
        }

        if !is_sky_flat(&sector.ceiling_texture) {
            for i in range(1, points.len()) {
                let (v1, v2) = (points[i], points[(i + 1) % points.len()]);
                self.flat_vertex(&v2, ceiling, bright, ceiling_bounds);
                self.flat_vertex(&v1, ceiling, bright, ceiling_bounds);
                self.flat_vertex(&v0, ceiling, bright, ceiling_bounds);
            }
        }
    }

    fn flat_vertex(&mut self, xz: &Vec2f, y: f32, brightness: f32,
                   bounds: &Bounds) {
        self.flats.push(FlatVertex {
            _pos: Vec3::new(xz.x, y, xz.y),
            _brightness: brightness,
            _offsets: bounds.pos,
            _num_frames: bounds.num_frames as u8,
            _frame_offset: bounds.frame_offset as u8,
        });
    }

    fn wall_vertex(&mut self, xz: &Vec2f, y: f32, tile_u: f32, tile_v: f32,
                   brightness: f32, scroll_rate: f32, bounds: &Bounds) {
        self.walls.push(WallVertex {
            _pos: Vec3::new(xz.x, y, xz.y),
            _tile_uv: Vec2::new(tile_u, tile_v),
            _atlas_uv: bounds.pos,
            _tile_width: bounds.size.x,
            _brightness: brightness,
            _scroll_rate: scroll_rate,
            _num_frames: bounds.num_frames as u8,
            _frame_offset: bounds.frame_offset as u8,
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
    let center = polygon_center(points.as_slice());
    points.sort_by(
        |a, b| {
            let ac = a - center;
            let bc = b - center;
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
        let next_point = (*points)[i_point % points.len()];
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
    while (simplified[0] - simplified[simplified.len() - 1]).norm() < 0.0032 {
        simplified.pop();
    }

    let center = polygon_center(simplified.as_slice());
    for point in simplified.iter_mut() {
        *point = *point + (*point - center).normalized() * POLY_BIAS;
    }
    *points = simplified;
}
