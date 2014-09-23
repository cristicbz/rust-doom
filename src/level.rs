use check_gl;
use gl;
use line::{Line2, Line2f};
use numvec::{Vec2f, Vec2, Vec3f, Vec3, Numvec};
use shader::{Shader, Uniform};
use mat4::Mat4;
use std::vec::Vec;
use std::str;
use vbo::VertexBuffer;
use wad;
use wad::util::{from_wad_height, from_wad_coords, is_untextured, is_sky_texture,
                parse_child_id, lower_name};
use wad::tex::TextureDirectory;
use wad::tex::Bounds;
use wad::types::*;
use libc::c_void;
use std::collections::{HashSet, HashMap};
use std::hash::sip::SipHasher;
use std::mem;
use texture::Texture;


static DRAW_WALLS : bool = true;
static WIRE_FLOORS : bool = false;
static BSP_TOLERANCE : f32 = 1e-3;
static SEG_TOLERANCE : f32 = 0.1;
static RENDER_SKY : bool = true;

fn should_render(name: &WadName) -> bool {
    !is_untextured(name) && (RENDER_SKY | !is_sky_texture(name))
}

pub struct Level {
    start_pos: Vec2f,

    flat_shader: Shader,
    flat_u_transform: Uniform,
    flat_u_palette: Uniform,
    flat_utexture: Uniform,
    flats_vbo: VertexBuffer,

    wall_shader: Shader,
    wall_u_transform: Uniform,
    wall_u_atlas_size: Uniform,
    wall_u_atlas: Uniform,
    wall_u_palette: Uniform,
    walls_vbo: VertexBuffer,

    palette: Texture,
    flat_atlas: Texture,
    wall_texture_atlas: Texture,
}

macro_rules! offset_of(
    ($T:ty, $m:ident) => ((&((*(0 as *const $T)).$m))
                          as *const _ as *const c_void)
)

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
            flats.insert(Vec::from_slice(sector.floor_texture));
            flats.insert(Vec::from_slice(sector.ceiling_texture));
        }
        let (flat_atlas, flat_lookup) =
            textures.build_flat_atlas(flats.len(), flats.iter()
                                                        .map(|x| x.as_slice()));
        let mut walls = HashSet::with_hasher(SipHasher::new());
        for sidedef in data.sidedefs.iter() {
            if should_render(&sidedef.upper_texture) {
                walls.insert(Vec::from_slice(sidedef.upper_texture));
            }
            if should_render(&sidedef.middle_texture) {
                walls.insert(Vec::from_slice(sidedef.middle_texture));
            }
            if should_render(&sidedef.lower_texture) {
                walls.insert(Vec::from_slice(sidedef.lower_texture));
            }
        }
        let (wall_texture_atlas, wall_lookup) =
            textures.build_wall_atlas(walls.iter().map(|x| x.as_slice()));

        let builder = VboBuilder::from_wad(
            &data, &flat_lookup, &wall_lookup);

        let flat_shader = Shader::new_from_files(
            &Path::new("src/shaders/flat.vertex.glsl"),
            &Path::new("src/shaders/flat.fragment.glsl")).unwrap();

        let wall_shader = Shader::new_from_files(
            &Path::new("src/shaders/wall.vertex.glsl"),
            &Path::new("src/shaders/wall.fragment.glsl")).unwrap();

        Level {
            start_pos: start_pos,

            flat_u_palette: flat_shader.expect_uniform("u_palette"),
            flat_utexture: flat_shader.expect_uniform("u_texture"),
            flat_u_transform: flat_shader.expect_uniform("u_transform"),
            flat_shader: flat_shader,
            flats_vbo: builder.bake_flats(),

            wall_u_transform: wall_shader.expect_uniform("u_transform"),
            wall_u_atlas_size: wall_shader.expect_uniform("u_atlas_size"),
            wall_u_atlas: wall_shader.expect_uniform("u_atlas"),
            wall_u_palette: wall_shader.expect_uniform("u_palette"),
            wall_shader: wall_shader,
            wall_texture_atlas: wall_texture_atlas,
            walls_vbo: builder.bake_walls(),

            palette: textures.build_palette_texture(0, 0, 31),
            flat_atlas: flat_atlas,
        }
    }

    pub fn get_start_pos<'a>(&'a self) -> &'a Vec2f { &self.start_pos }

    pub fn render_flats(&self, projection_view: &Mat4) {
        self.palette.bind(gl::TEXTURE0);
        self.flat_atlas.bind(gl::TEXTURE1);

        self.flat_shader.bind();
        self.flat_shader.set_uniform_i32(self.flat_u_palette, 0);
        self.flat_shader.set_uniform_i32(self.flat_utexture, 1);
        self.flat_shader.set_uniform_mat4(self.flat_u_transform,
                                          projection_view);
        check_gl!(gl::EnableVertexAttribArray(0));
        check_gl!(gl::EnableVertexAttribArray(1));
        check_gl!(gl::EnableVertexAttribArray(2));
        self.flats_vbo.bind();

        let stride = mem::size_of::<FlatVertex>() as i32;
        let pos_offset = 0 as *const c_void;
        let offsets_offset = 12 as *const c_void;
        let brightness_offset = 20 as *const c_void;
        check_gl_unsafe!(gl::VertexAttribPointer(
                0, 3, gl::FLOAT, gl::FALSE, stride, pos_offset));
        check_gl_unsafe!(gl::VertexAttribPointer(
                1, 2, gl::FLOAT, gl::FALSE, stride, offsets_offset));
        check_gl_unsafe!(gl::VertexAttribPointer(
                2, 1, gl::FLOAT, gl::FALSE, stride, brightness_offset));

        check_gl!(gl::DrawArrays(gl::TRIANGLES, 0,
                                 self.flats_vbo.len() as i32));
        self.flats_vbo.unbind();
        check_gl!(gl::DisableVertexAttribArray(0));
        check_gl!(gl::DisableVertexAttribArray(1));
        check_gl!(gl::DisableVertexAttribArray(2));
        self.flat_shader.unbind();

        self.flat_atlas.unbind(gl::TEXTURE1);
        self.palette.unbind(gl::TEXTURE0);
    }

    pub fn render_walls(&self, projection_view: &Mat4) {
        self.palette.bind(gl::TEXTURE0);
        self.wall_texture_atlas.bind(gl::TEXTURE1);
        self.wall_shader.bind();
        self.wall_shader.set_uniform_mat4(self.wall_u_transform,
                                          projection_view);
        self.wall_shader.set_uniform_i32(self.wall_u_palette, 0);
        self.wall_shader.set_uniform_i32(self.wall_u_atlas, 1);
        self.wall_shader.set_uniform_f32(
            self.wall_u_atlas_size, self.wall_texture_atlas.get_width() as f32);

        check_gl!(gl::EnableVertexAttribArray(0));
        check_gl!(gl::EnableVertexAttribArray(1));
        check_gl!(gl::EnableVertexAttribArray(2));
        check_gl!(gl::EnableVertexAttribArray(3));
        check_gl!(gl::EnableVertexAttribArray(4));
        self.walls_vbo.bind();

        let stride = mem::size_of::<WallVertex>() as i32;
        check_gl_unsafe!(
            gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, stride,
                                    offset_of!(WallVertex, pos)));
        check_gl_unsafe!(
            gl::VertexAttribPointer(1, 2, gl::FLOAT, gl::FALSE, stride,
                                    offset_of!(WallVertex, tile_uv)));
        check_gl_unsafe!(
            gl::VertexAttribPointer(2, 2, gl::FLOAT, gl::FALSE, stride,
                                    offset_of!(WallVertex, atlas_uv)));
        check_gl_unsafe!(
            gl::VertexAttribPointer(3, 1, gl::FLOAT, gl::FALSE, stride,
                                    offset_of!(WallVertex, tile_width)));
        check_gl_unsafe!(
            gl::VertexAttribPointer(4, 1, gl::FLOAT, gl::FALSE, stride,
                                    offset_of!(WallVertex, brightness)));
        check_gl!(gl::DrawArrays(gl::TRIANGLES, 0,
                                 self.walls_vbo.len() as i32));
        self.walls_vbo.unbind();
        check_gl!(gl::DisableVertexAttribArray(0));
        check_gl!(gl::DisableVertexAttribArray(1));
        check_gl!(gl::DisableVertexAttribArray(2));
        check_gl!(gl::DisableVertexAttribArray(3));
        check_gl!(gl::DisableVertexAttribArray(4));
        self.wall_shader.unbind();

        self.palette.unbind(gl::TEXTURE0);
        self.wall_texture_atlas.unbind(gl::TEXTURE1);
    }

    pub fn render(&self, projection_view: &Mat4) {
        //check_gl!(gl::PolygonMode(gl::FRONT_AND_BACK, gl::LINE));
        check_gl!(gl::Enable(gl::CULL_FACE));
        self.render_flats(projection_view);
        self.render_walls(projection_view);
    }
}

#[repr(packed)]
struct FlatVertex {
    pub pos: Vec3f,
    pub offsets: Vec2f,
    pub brightness: f32,
}

#[repr(packed)]
struct WallVertex {
    pub pos: Vec3f,
    pub tile_uv: Vec2f,
    pub atlas_uv: Vec2f,
    pub tile_width: f32,
    pub brightness: f32,
}

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
        VertexBuffer::new_with_data(gl::ARRAY_BUFFER,
                                    gl::STATIC_DRAW, self.flat_data.as_slice())
    }

    pub fn bake_walls(&self) -> VertexBuffer {
        VertexBuffer::new_with_data(gl::ARRAY_BUFFER,
                                    gl::STATIC_DRAW, self.wall_data.as_slice())
    }

    fn push_wall_seg(&mut self, seg: &WadSeg) {
        let wad = self.wad;
        let side = wad.seg_sidedef(seg);
        let sector = wad.seg_sector(seg);
        let (floor, ceil) = (sector.floor_height, sector.ceiling_height);

        let back_sector = match wad.seg_back_sector(seg) {
            None => {
                self.push_seg(
                    seg, (floor, ceil), sector.light, &side.middle_texture);
                return
            },
            Some(s) => s
        };

        let back_floor = back_sector.floor_height;
        let back_ceil = back_sector.ceiling_height;

        let floor = if back_floor > floor {
            self.push_seg(seg, (floor, back_floor), sector.light,
                          &side.lower_texture);
            back_floor
        } else {
            floor
        };
        let ceil = if back_ceil < ceil {
            self.push_seg(seg, (back_ceil, ceil), sector.light,
                          &side.upper_texture);
            back_ceil
        } else {
            ceil
        };
        self.push_seg(seg, (floor, ceil), sector.light, &side.middle_texture);

    }

    fn flat_vertex(&mut self, x: f32, y: f32, z: f32,
                   brightness: f32, offsets: &Vec2f) {
        self.flat_data.push(FlatVertex {
            pos: Vec3::new(x, y, z),
            brightness: brightness,
            offsets: *offsets
        });
    }

    fn wall_vertex(&mut self, xz: &Vec2f, y: f32, tile_u: f32, tile_v: f32,
                   brightness: f32, bounds: &Bounds) {
        self.wall_data.push(WallVertex {
            pos: Vec3::new(xz.x, y, xz.y),
            tile_uv: Vec2::new(tile_u, tile_v),
            atlas_uv: bounds.pos,
            tile_width: bounds.size.x,
            brightness: brightness,
        });
    }

    fn push_seg(&mut self, seg: &WadSeg,
                (low, high): (WadCoord, WadCoord),
                brightness: i16, texture_name: &[u8, ..8]) {
        if !DRAW_WALLS { return; }
        if !should_render(texture_name) { return; }
        let bounds = self.wall_lookup.find(&lower_name(texture_name))
            .or_else(|| {
                fail!("push_seg: No such wall texture '{}'",
                      str::from_utf8(texture_name));
            }).unwrap();

        let brightness = brightness as f32 / 256.0;
        let (v1, v2) = (self.wad.vertex(seg.start_vertex),
                        self.wad.vertex(seg.end_vertex));
        let (low, high) = (from_wad_height(low), from_wad_height(high));

        let side = self.wad.seg_sidedef(seg);
        let s1 = seg.offset as f32 + side.x_offset as f32;
        let s2 = s1 + (v2 - v1).norm() * 100.0;
        let t2 = 0.0;
        let t1 = (high - low) * 100.0;

        self.wall_vertex(&v1, low,  s1, t1, brightness, bounds);
        self.wall_vertex(&v2, low,  s2, t1, brightness, bounds);
        self.wall_vertex(&v1, high, s1, t2, brightness, bounds);

        self.wall_vertex(&v2, low,  s2, t1, brightness, bounds);
        self.wall_vertex(&v2, high, s2, t2, brightness, bounds);
        self.wall_vertex(&v1, high, s1, t2, brightness, bounds);
    }

    fn wire_floor(&mut self, _sector: &WadSector, _points: &[Vec2f]) {
        //let center = polygon_center(points);
        //let v1 = center - Vec2::new(0.03, 0.03);
        //let v2 = center + Vec2::new(0.03, 0.03);
        //let y = if !ZERO_FLOORS { from_wad_height(sector.floor_height) }
        //        else { 0.0 };

        //self.flat_vertex(v1.x, y, v2.y, 1.0);
        //self.flat_vertex(v2.x, y, v1.y, 1.0);
        //self.flat_vertex(v1.x, y, v1.y, 1.0);

        //self.flat_vertex(v1.x, y, v2.y, 1.0);
        //self.flat_vertex(v2.x, y, v2.y, 1.0);
        //self.flat_vertex(v2.x, y, v1.y, 1.0);

        //for i in range(0, points.len()) {
        //    let (v1, v2) = (points[i], points[(i + 1) % points.len()]);
        //    let t = 3.0;
        //    let e = (v1 - v2).normalized() * 0.02;
        //    let n = e.normal();
        //    let (v1, v2) = (v1 - e, v2 + e);

        //    self.flat_vertex(v2.x + n.x*t, y, v2.y + n.y*t, 1.0);
        //    self.flat_vertex(v1.x + n.x*t, y, v1.y + n.y*t, 1.0);
        //    self.flat_vertex(v1.x + n.x, y, v1.y + n.y, 1.0);

        //    self.flat_vertex(v1.x + n.x, y, v1.y + n.y, 1.0);
        //    self.flat_vertex(v2.x + n.x, y, v2.y + n.y, 1.0);
        //    self.flat_vertex(v2.x + n.x*t, y, v2.y + n.y*t, 1.0);
        //}
    }

    fn convex_flat(&mut self, sector: &WadSector, points: &[Vec2f]) {
        let floor = from_wad_height(sector.floor_height);
        let ceiling = from_wad_height(sector.ceiling_height);
        let v0 = points[0];
        let floor_offsets = self.flat_lookup.find(
                &lower_name(sector.floor_texture))
            .expect("convex_flat: No such floor texture.");
        let ceiling_offsets = self.flat_lookup.find(
                &lower_name(sector.ceiling_texture))
            .expect("convex_flat: No such ceiling texture.");
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
            self.push_wall_seg(seg);
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
        if WIRE_FLOORS {
            self.wire_floor(sector, points.as_slice());
        } else {
            self.convex_flat(sector, points.as_slice());
        }
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
