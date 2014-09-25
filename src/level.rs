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
                name_toupper};


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

        let mut flats = HashSet::with_hasher(SipHasher::new());
        for sector in data.sectors.iter() {
            flats.insert(name_toupper(sector.floor_texture));
            flats.insert(name_toupper(sector.ceiling_texture));
        }
        let (flat_atlas, flat_lookup) =
            textures.build_flat_atlas(flats.len(), flats.iter()
                                                        .map(|x| x.as_slice()));
        let mut walls = HashSet::with_hasher(SipHasher::new());
        for sidedef in data.sidedefs.iter() {
            if should_render(&sidedef.upper_texture) {
                walls.insert(name_toupper(sidedef.upper_texture));
            }
            if should_render(&sidedef.middle_texture) {
                walls.insert(name_toupper(sidedef.middle_texture));
            }
            if should_render(&sidedef.lower_texture) {
                walls.insert(name_toupper(sidedef.lower_texture));
            }
        }
        let (wall_atlas, wall_lookup) =
            textures.build_wall_atlas(walls.iter().map(|x| x.as_slice()));

        let builder = VboBuilder::from_wad(
            &data, &flat_lookup, &wall_lookup);

        let flat_shader = Shader::new_from_files(
            &Path::new("src/shaders/flat.vertex.glsl"),
            &Path::new("src/shaders/flat.fragment.glsl")).unwrap();

        let wall_shader = Shader::new_from_files(
            &Path::new("src/shaders/wall.vertex.glsl"),
            &Path::new("src/shaders/wall.fragment.glsl")).unwrap();

        let palette_texture = Rc::new(textures.build_palette_texture(0, 0, 31));

        let mut flats_step = RenderStep::new(flat_shader);
        flats_step
            .add_shared_texture("u_palette", palette_texture.clone(), 0)
            .add_unique_texture("u_texture", flat_atlas, 1)
            .add_static_vbo(builder.bake_flats());

        let mut walls_step = RenderStep::new(wall_shader);
        walls_step
            .add_shared_texture("u_palette", palette_texture, 0)
            .add_constant_f32("u_atlas_size", wall_atlas.get_width() as f32)
            .add_unique_texture("u_atlas", wall_atlas, 1)
            .add_static_vbo(builder.bake_walls());


        let mut renderer = Renderer::new();
        renderer.add_step(flats_step);
        renderer.add_step(walls_step);

        Level {
            start_pos: start_pos,
            renderer: renderer,
        }
    }

    pub fn get_start_pos<'a>(&'a self) -> &'a Vec2f { &self.start_pos }

    pub fn render(&mut self, delta_time: f32, projection_view: &Mat4) {
        self.renderer.render(delta_time, projection_view);
    }
}


static BSP_TOLERANCE : f32 = 1e-3;
static SEG_TOLERANCE : f32 = 0.1;


macro_rules! offset_of(
    ($T:ty, $m:ident) =>
        (unsafe { (&((*(0 as *const $T)).$m)) as *const _ as *const c_void })
)


pub struct VboBuilder<'a> {
    wad: &'a wad::Level,
    flat_lookup: &'a HashMap<Vec<u8>, Vec2f>,
    wall_lookup: &'a HashMap<Vec<u8>, Bounds>,
    flat_data: Vec<FlatVertex>,
    wall_data: Vec<WallVertex>,
}
impl<'a> VboBuilder<'a> {
    pub fn from_wad(lvl: &'a wad::Level,
                    flat_lookup: &'a HashMap<Vec<u8>, Vec2f>,
                    wall_lookup: &'a HashMap<Vec<u8>, Bounds>)
            -> VboBuilder<'a> {
        let mut new = VboBuilder { wad: lvl,
                                   wall_data: Vec::new(),
                                   flat_lookup: flat_lookup,
                                   wall_lookup: wall_lookup,
                                   flat_data: Vec::new() };
        new.push_node(lvl.nodes.last().unwrap(), &mut Vec::new());
        new
    }

    pub fn bake_flats(&self) -> VertexBuffer {
        let mut buffer = BufferBuilder::<FlatVertex>::new(3)
            .attribute_vec3f(0, offset_of!(FlatVertex, _pos))
            .attribute_vec2f(1, offset_of!(FlatVertex, _offsets))
            .attribute_f32(2, offset_of!(FlatVertex, _brightness))
            .build();
        buffer.set_data(gl::STATIC_DRAW, self.flat_data.as_slice());
        buffer
    }

    pub fn bake_walls(&self) -> VertexBuffer {
        let mut buffer = BufferBuilder::<WallVertex>::new(6)
            .attribute_vec3f(0, offset_of!(WallVertex, _pos))
            .attribute_vec2f(1, offset_of!(WallVertex, _tile_uv))
            .attribute_vec2f(2, offset_of!(WallVertex, _atlas_uv))
            .attribute_f32(3, offset_of!(WallVertex, _tile_width))
            .attribute_f32(4, offset_of!(WallVertex, _brightness))
            .attribute_f32(5, offset_of!(WallVertex, _scroll_rate))
            .build();
        buffer.set_data(gl::STATIC_DRAW, self.wall_data.as_slice());
        buffer
    }

    fn push_node(&mut self, node: &WadNode, lines: &mut Vec<Line2f>) {
        let (left, leaf_left) = parse_child_id(node.left);
        let (right, leaf_right) = parse_child_id(node.right);
        let partition = Line2::from_origin_and_displace(
            from_wad_coords(node.line_x, node.line_y),
            from_wad_coords(node.step_x, node.step_y));

        lines.push(partition);
        if leaf_left {
            self.push_subsector(&self.wad.subsectors[left], lines.as_slice());
        } else {
            self.push_node(&self.wad.nodes[left], lines);
        }
        lines.pop();


        lines.push(partition.inverted_halfspaces());
        if leaf_right {
            self.push_subsector(&self.wad.subsectors[right], lines.as_slice());
        } else {
            self.push_node(&self.wad.nodes[right], lines);
        }
        lines.pop();
    }

    fn push_subsector(&mut self, subsector: &WadSubsector, lines: &[Line2f]) {
        let segs = self.wad.ssector_segs(subsector);

        // The vector contains all (2D) points which are part of the subsector:
        // implicit (intersection of BSP lines) and explicit (seg vertices).
        let mut points : Vec<Vec2f> = Vec::new();

        // First add the explicit points.
        for seg in segs.iter() {
            let (v1, v2) = self.wad.seg_vertices(seg);
            points.push(v1);
            points.push(v2);

            // Also push the wall segments.
            self.push_seg(seg);
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

                let line_dist = |l : &Line2f| l.signed_distance(&point);
                let seg_dist = |s : &WadSeg|
                    Line2::from_point_pair(self.wad.seg_vertices(s))
                        .signed_distance(&point);

                // The intersection point must lie both within the BSP volume
                // and the segs volume.
                if lines.iter().map(line_dist).all(|d| d >= -BSP_TOLERANCE) &&
                   segs.iter().map(seg_dist).all(|d| d <= SEG_TOLERANCE) {
                    points.push(point);
                }
            }
        }

        points_to_polygon(&mut points);  // Sort and remove duplicates.
        let sector = self.wad.seg_sector(&segs[0]);
        self.push_flat_poly(sector, points.as_slice());
    }


    fn push_seg(&mut self, seg: &WadSeg) {
        let wad = self.wad;
        let line = wad.seg_linedef(seg);
        let side = wad.seg_sidedef(seg);
        let sector = wad.seg_sector(seg);
        let (floor, ceil) = (sector.floor_height, sector.ceiling_height);
        let (unpeg_lower, unpeg_upper) = (line.lower_unpegged(),
                                          line.upper_unpegged());

        let back_sector = match wad.seg_back_sector(seg) {
            None => {
                self.push_wall_quad(seg, (floor, ceil), sector.light,
                                    &side.middle_texture,
                                    if unpeg_lower { PegBottom }
                                    else { PegTop });
                return
            },
            Some(s) => s
        };

        let back_floor = back_sector.floor_height;
        let back_ceil = back_sector.ceiling_height;

        let floor = if back_floor > floor {
            self.push_wall_quad(seg, (floor, back_floor), sector.light,
                                &side.lower_texture,
                                if unpeg_lower { PegBottomLower }
                                else { PegTop });
            back_floor
        } else {
            floor
        };
        let ceil = if back_ceil < ceil {
            self.push_wall_quad(seg, (back_ceil, ceil), sector.light,
                                &side.upper_texture,
                                if unpeg_upper { PegTop }
                                else { PegBottom });
            back_ceil
        } else {
            ceil
        };
        self.push_wall_quad(seg, (floor, ceil), sector.light, &side.middle_texture,
                            if unpeg_lower { PegBottom } else { PegTop });

    }

    fn push_wall_quad(&mut self, seg: &WadSeg,
                      (low, high): (WadCoord, WadCoord),
                      brightness: i16, texture_name: &[u8, ..8],
                      peg: PegType) {
        if !should_render(texture_name) { return; }
        let bounds = self.wall_lookup.find(&name_toupper(texture_name))
            .or_else(|| {
                fail!("push_seg_quad: No such wall texture '{}'",
                      str::from_utf8(texture_name));
            }).unwrap();

        let brightness = brightness as f32 / 256.0;
        let (v1, v2) = (self.wad.vertex(seg.start_vertex),
                        self.wad.vertex(seg.end_vertex));
        let (low, high) = (from_wad_height(low), from_wad_height(high));

        let line = self.wad.seg_linedef(seg);
        let side = self.wad.seg_sidedef(seg);
        let height = (high - low) * 100.0;
        let s1 = seg.offset as f32 + side.x_offset as f32;
        let s2 = s1 + (v2 - v1).norm() * 100.0;
        let (t1, t2) = match peg {
            PegTop => (height, 0.0),
            PegBottom => (bounds.size.y, bounds.size.y - height),
            PegBottomLower => {
                // As far as I can tell, this is a special case.
                let sector = self.wad.sidedef_sector(side);
                let sector_height = (sector.ceiling_height -
                                     sector.floor_height) as f32;
                (bounds.size.y + sector_height,
                 bounds.size.y - height + sector_height)
            }

        };
        let (t1, t2) = (t1 + side.y_offset as f32, t2 + side.y_offset as f32);

        let scroll = if line.special_type == 0x30 {
            35.0
        } else {
            0.0
        };

        self.wall_vertex(&v1, low,  s1, t1, brightness, scroll, bounds);
        self.wall_vertex(&v2, low,  s2, t1, brightness, scroll, bounds);
        self.wall_vertex(&v1, high, s1, t2, brightness, scroll, bounds);

        self.wall_vertex(&v2, low,  s2, t1, brightness, scroll, bounds);
        self.wall_vertex(&v2, high, s2, t2, brightness, scroll, bounds);
        self.wall_vertex(&v1, high, s1, t2, brightness, scroll, bounds);
    }

    fn push_flat_poly(&mut self, sector: &WadSector, points: &[Vec2f]) {
        let floor = from_wad_height(sector.floor_height);
        let ceiling = from_wad_height(sector.ceiling_height);
        let v0 = points[0];
        let floor_offsets = self.flat_lookup.find(
                &name_toupper(sector.floor_texture))
            .expect("push_flat_poly: No such floor texture.");
        let ceiling_offsets = self.flat_lookup.find(
                &name_toupper(sector.ceiling_texture))
            .expect("push_flat_poly: No such ceiling texture.");
        let bright = sector.light as f32 / 256.0 ;
        for i in range(1, points.len()) {
            let (v1, v2) = (points[i], points[(i + 1) % points.len()]);
            self.flat_vertex(v0.x, floor, v0.y, bright, floor_offsets);
            self.flat_vertex(v1.x, floor, v1.y, bright, floor_offsets);
            self.flat_vertex(v2.x, floor, v2.y, bright, floor_offsets);

            self.flat_vertex(v2.x, ceiling, v2.y, bright, ceiling_offsets);
            self.flat_vertex(v1.x, ceiling, v1.y, bright, ceiling_offsets);
            self.flat_vertex(v0.x, ceiling, v0.y, bright, ceiling_offsets);
        }
    }

    fn flat_vertex(&mut self, x: f32, y: f32, z: f32,
                   brightness: f32, offsets: &Vec2f) {
        self.flat_data.push(FlatVertex {
            _pos: Vec3::new(x, y, z),
            _brightness: brightness,
            _offsets: *offsets
        });
    }

    fn wall_vertex(&mut self, xz: &Vec2f, y: f32, tile_u: f32, tile_v: f32,
                   brightness: f32, scroll_rate: f32, bounds: &Bounds) {
        self.wall_data.push(WallVertex {
            _pos: Vec3::new(xz.x, y, xz.y),
            _tile_uv: Vec2::new(tile_u, tile_v),
            _atlas_uv: bounds.pos,
            _tile_width: bounds.size.x,
            _brightness: brightness,
            _scroll_rate: scroll_rate,
        });
    }
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


fn should_render(name: &WadName) -> bool { !is_untextured(name) }


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
