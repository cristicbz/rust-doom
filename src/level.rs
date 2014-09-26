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
        let data = wad::Level::from_archive(wad, name);
        info!("Building level {}...", str::from_utf8(name));

        let mut start_pos = Vec2::zero();
        for thing in data.things.iter() {
            if thing.thing_type == 1 {  // Player 1 start position.
                start_pos = from_wad_coords(thing.x, thing.y);
                info!("Player start position: {}.", start_pos);
            }
        }

        Level {
            start_pos: start_pos,
            renderer: build_level(wad, textures, name),
        }
    }

    pub fn get_start_pos<'a>(&'a self) -> &'a Vec2f { &self.start_pos }

    pub fn render(&mut self, delta_time: f32, projection_view: &Mat4) {
        self.renderer.render(delta_time, projection_view);
    }
}


type OffsetLookup = HashMap<Vec<u8>, Vec2f>;
type BoundsLookup = HashMap<Vec<u8>, Bounds>;


struct TextureMaps {
    flats: OffsetLookup,
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
}


#[repr(packed)]
struct WallVertex {
    _pos: Vec3f,
    _tile_uv: Vec2f,
    _atlas_uv: Vec2f,
    _tile_width: f32,
    _brightness: f32,
    _scroll_rate: f32,
}


static BSP_TOLERANCE : f32 = 1e-3;
static SEG_TOLERANCE : f32 = 0.1;

static PALETTE_UNIT: uint = 0;
static ATLAS_UNIT: uint = 1;


macro_rules! offset_of(
    ($T:ty, $m:ident) =>
        (unsafe { (&((*(0 as *const $T)).$m)) as *const _ as *const c_void })
)


pub fn build_level(wad: &mut wad::Archive,
                   textures: &wad::TextureDirectory,
                   level_name: &WadName)
        -> Renderer {
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
    renderer
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
                     step: &mut RenderStep) -> OffsetLookup {
    let mut flats = HashSet::with_hasher(SipHasher::new());
    for sector in level.sectors.iter() {
        flats.insert(name_toupper(sector.floor_texture));
        flats.insert(name_toupper(sector.ceiling_texture));
    }
    let (atlas, lookup) = textures.build_flat_atlas(
        flats.len(), flats.iter().map(|x| x.as_slice()));
    step.add_unique_texture("u_texture", atlas, ATLAS_UNIT);
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
    let (atlas, lookup) = textures.build_wall_atlas(
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
        let buffer = BufferBuilder::<FlatVertex>::new(3)
            .attribute_vec3f(0, offset_of!(FlatVertex, _pos))
            .attribute_vec2f(1, offset_of!(FlatVertex, _offsets))
            .attribute_f32(2, offset_of!(FlatVertex, _brightness))
            .build();
        buffer
    }

    fn create_walls_buffer() -> VertexBuffer {
        let buffer = BufferBuilder::<WallVertex>::new(6)
            .attribute_vec3f(0, offset_of!(WallVertex, _pos))
            .attribute_vec2f(1, offset_of!(WallVertex, _tile_uv))
            .attribute_vec2f(2, offset_of!(WallVertex, _atlas_uv))
            .attribute_f32(3, offset_of!(WallVertex, _tile_width))
            .attribute_f32(4, offset_of!(WallVertex, _brightness))
            .attribute_f32(5, offset_of!(WallVertex, _scroll_rate))
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
                fail!("push_seg_quad: No such wall texture '{}'",
                      str::from_utf8(texture_name));
            },
            Some(bounds) => bounds,
        };

        let line = self.level.seg_linedef(seg);
        let side = self.level.seg_sidedef(seg);
        let sector = self.level.sidedef_sector(side);
        let (v1, v2) = self.level.seg_vertices(seg);
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
        let floor_offsets = self.bounds.flats.find(
                &name_toupper(sector.floor_texture))
            .expect("push_flat_poly: No such floor texture.");
        let ceiling_offsets = self.bounds.flats.find(
                &name_toupper(sector.ceiling_texture))
            .expect("push_flat_poly: No such ceiling texture.");
        let bright = sector.light as f32 / 255.0;
        let v0 = points[0];
        for i in range(1, points.len()) {
            let (v1, v2) = (points[i], points[(i + 1) % points.len()]);
            self.flat_vertex(v0.x, floor, v0.y, bright, floor_offsets);
            self.flat_vertex(v1.x, floor, v1.y, bright, floor_offsets);
            self.flat_vertex(v2.x, floor, v2.y, bright, floor_offsets);
        }

        if is_sky_flat(&sector.ceiling_texture) { return; }
        for i in range(1, points.len()) {
            let (v1, v2) = (points[i], points[(i + 1) % points.len()]);
            self.flat_vertex(v2.x, ceiling, v2.y, bright, ceiling_offsets);
            self.flat_vertex(v1.x, ceiling, v1.y, bright, ceiling_offsets);
            self.flat_vertex(v0.x, ceiling, v0.y, bright, ceiling_offsets);
        }
    }

    fn flat_vertex(&mut self, x: f32, y: f32, z: f32, brightness: f32,
                   offsets: &Vec2f) {
        self.flats.push(FlatVertex {
            _pos: Vec3::new(x, y, z),
            _brightness: brightness,
            _offsets: *offsets
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
    let mut n_unique = 1;
    for i_point in range(1, points.len()) {
        let d = (*points)[i_point] - (*points)[i_point - 1];
        if d.x.abs() > 1e-10 || d.y.abs() > 1e-10 {
            *points.get_mut(n_unique) = (*points)[i_point];
            n_unique = n_unique + 1;
        }
    }
    points.truncate(n_unique);
}
