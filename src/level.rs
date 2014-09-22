use check_gl;
use gl;
use line::{Line2, Line2f};
use numvec::{Vec2f, Vec2, Vec3f, Vec3, Numvec};
use shader::{Shader, Uniform};
use mat4::Mat4;
use std::vec::Vec;
use std::str;
use std::string::String;
use std::ptr;
use vbo::VertexBuffer;
use wad;
use wad::util::{from_wad_height, from_wad_coords, is_untextured, is_sky_texture,
                parse_child_id};
use wad::types::*;
use libc::c_void;
use std::collections::HashSet;


static DRAW_WALLS : bool = true;
static WIRE_FLOORS : bool = false;
static ZERO_FLOORS : bool = false;
static BSP_TOLERANCE : f32 = 1e-3;
static SEG_TOLERANCE : f32 = 0.1;
static RENDER_SKY : bool = true;

fn should_render(name: &WadName) -> bool {
    !is_untextured(name) && (RENDER_SKY | !is_sky_texture(name))
}


pub struct Level {
    start_pos: Vec2f,

    flat_shader: Shader,
    flat_uniform_transform: Uniform,
    flat_uniform_eye: Uniform,
    flats_vbo: VertexBuffer,

    wall_shader: Shader,
    wall_uniform_transform: Uniform,
    wall_uniform_eye: Uniform,
    walls_vbo: VertexBuffer,
}


impl Level {
    pub fn new(wad: &mut wad::Archive, name: &WadName) -> Level {
        let data = wad::Level::from_archive(wad, name);
        info!("Building level {}...", str::from_utf8(name));

        let mut start_pos = Vec2::zero();
        for thing in data.things.iter() {
            if thing.thing_type == 1 {  // Player 1 start position.
                start_pos = from_wad_coords(thing.x, thing.y);
                info!("Player start position: {}.", start_pos);
            }
        }

        let mut textures = HashSet::new();
        for sidedef in data.sidedefs.iter() {
            textures.insert(String::from_str(
                    str::from_utf8(sidedef.upper_texture).unwrap()));
            textures.insert(String::from_str(
                    str::from_utf8(sidedef.lower_texture).unwrap()));
            textures.insert(String::from_str(
                    str::from_utf8(sidedef.middle_texture).unwrap()));
        }
        info!("  {} static textures.", textures.len());

        let flat_shader = Shader::new_from_files(
            &Path::new("src/shaders/flat.vertex.glsl"),
            &Path::new("src/shaders/flat.fragment.glsl")).unwrap();

        let wall_shader = Shader::new_from_files(
            &Path::new("src/shaders/wall.vertex.glsl"),
            &Path::new("src/shaders/wall.fragment.glsl")).unwrap();

        let builder = VboBuilder::from_wad(&data);

        Level {
            start_pos: start_pos,

            flat_uniform_transform: flat_shader.get_uniform("u_transform").unwrap(),
            flat_uniform_eye: flat_shader.get_uniform("u_eye").unwrap(),
            flat_shader: flat_shader,
            flats_vbo: builder.bake_flats(),

            wall_uniform_transform: wall_shader.get_uniform("u_transform").unwrap(),
            wall_uniform_eye: wall_shader.get_uniform("u_eye").unwrap(),
            wall_shader: wall_shader,
            walls_vbo: builder.bake_walls(),
        }
    }

    pub fn get_start_pos<'a>(&'a self) -> &'a Vec2f { &self.start_pos }

    pub fn render_flats(&self, projection_view: &Mat4, eye: &Vec3f) {
        self.flat_shader.bind();
        self.flat_shader.set_uniform_mat4(self.flat_uniform_transform,
                                          projection_view);
        self.flat_shader.set_uniform_vec3f(self.flat_uniform_eye, eye);
        check_gl!(gl::EnableVertexAttribArray(0));
        check_gl!(gl::EnableVertexAttribArray(1));
        self.flats_vbo.bind();
        check_gl_unsafe!(gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE,
                                                 24, ptr::null()));
        check_gl_unsafe!(gl::VertexAttribPointer(1, 3, gl::FLOAT, gl::FALSE,
                                                 24, 12 as *const c_void));
        check_gl!(gl::DrawArrays(gl::TRIANGLES, 0,
                                 self.flats_vbo.len() as i32));
        self.flats_vbo.unbind();
        check_gl!(gl::DisableVertexAttribArray(0));
        check_gl!(gl::DisableVertexAttribArray(1));
        self.flat_shader.unbind();
    }

    pub fn render_walls(&self, projection_view: &Mat4, eye: &Vec3f) {
        self.wall_shader.bind();
        self.wall_shader.set_uniform_mat4(self.flat_uniform_transform,
                                          projection_view);
        self.wall_shader.set_uniform_vec3f(self.flat_uniform_eye, eye);
        check_gl!(gl::EnableVertexAttribArray(0));
        check_gl!(gl::EnableVertexAttribArray(1));
        self.walls_vbo.bind();
        check_gl_unsafe!(gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE,
                                                 24, ptr::null()));
        check_gl_unsafe!(gl::VertexAttribPointer(1, 3, gl::FLOAT, gl::FALSE,
                                                 24, 12 as *const c_void));
        check_gl!(gl::DrawArrays(gl::TRIANGLES, 0,
                                 self.walls_vbo.len() as i32));
        self.walls_vbo.unbind();
        check_gl!(gl::DisableVertexAttribArray(0));
        check_gl!(gl::DisableVertexAttribArray(1));
        self.wall_shader.unbind();
    }

    pub fn render(&self, projection_view: &Mat4, eye: &Vec3f) {
        //check_gl!(gl::PolygonMode(gl::FRONT_AND_BACK, gl::LINE));
        check_gl!(gl::Enable(gl::CULL_FACE));
        self.render_flats(projection_view, eye);
        self.render_walls(projection_view, eye);
    }
}

#[repr(packed)]
struct FlatVertex {
    pub pos: Vec3f,
    pub normal: Vec3f,
}

#[repr(packed)]
struct WallVertex {
    pub pos: Vec3f,
    pub normal: Vec3f,
}

pub struct VboBuilder<'a> {
    wad: &'a wad::Level,
    flat_data: Vec<FlatVertex>,
    wall_data: Vec<WallVertex>,
}

impl<'a> VboBuilder<'a> {
    pub fn from_wad(lvl: &'a wad::Level) -> VboBuilder<'a> {
        let mut new = VboBuilder { wad: lvl,
                                   wall_data: Vec::new(),
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
        let lvl = self.wad;
        let linedef = lvl.seg_linedef(seg);
        if linedef.left_side == -1 {
            let side = lvl.right_sidedef(linedef);
            if should_render(&side.middle_texture) {
                let sector = lvl.sidedef_sector(side);
                self.push_seg(seg,
                              (sector.floor_height, sector.ceiling_height));
            }
        } else if linedef.right_side == -1 {
            let side = lvl.left_sidedef(linedef);
            if should_render(&side.middle_texture) {
                let sector = lvl.sidedef_sector(side);
                self.push_seg(seg,
                              (sector.floor_height, sector.ceiling_height));
            }
        } else {
            let lside = lvl.left_sidedef(linedef);
            let rside = lvl.right_sidedef(linedef);
            let lsect = lvl.sidedef_sector(lside);
            let rsect = lvl.sidedef_sector(rside);
            let (lfloor, rfloor) = (lsect.floor_height, rsect.floor_height);
            let (lceil, rceil) = (lsect.ceiling_height, rsect.ceiling_height);

            if lfloor < rfloor {
                if should_render(&lside.lower_texture) {
                    self.push_seg(seg, (lfloor, rfloor));
                }
            } else if lfloor > rfloor {
                if should_render(&rside.lower_texture) {
                    self.push_seg(seg, (rfloor, lfloor));
                }
            }

            if lceil < rceil {
                if should_render(&rside.upper_texture) {
                    self.push_seg(seg, (lceil, rceil))
                }
            } else if lceil > rceil {
                if should_render(&lside.upper_texture) {
                    self.push_seg(seg, (rceil, lceil))
                }
            }

            if should_render(&lside.middle_texture) {
                self.push_seg(seg, (lfloor, lceil));
            }
            if should_render(&rside.middle_texture) {
                self.push_seg(seg, (rfloor, rceil));
            }
        }
    }

    fn flat_vertex(&mut self, x: f32, y: f32, z: f32, normal_y: f32) {
        self.flat_data.push(FlatVertex {
            pos: Vec3::new(x, y, z), normal: Vec3::new(0.0, normal_y, 0.0) });
    }

    fn wall_vertex(&mut self, xz: &Vec2f, y: f32, normal: &Vec2f) {
        self.wall_data.push(WallVertex {
            pos: Vec3::new(xz.x, y, xz.y),
            normal: Vec3::new(normal.x, 0.0, normal.y)
        });
    }

    fn wire_floor(&mut self, sector: &WadSector, points: &[Vec2f]) {
        let center = polygon_center(points);
        let v1 = center - Vec2::new(0.03, 0.03);
        let v2 = center + Vec2::new(0.03, 0.03);
        let y = if !ZERO_FLOORS { from_wad_height(sector.floor_height) }
                else { 0.0 };

        self.flat_vertex(v1.x, y, v2.y, 1.0);
        self.flat_vertex(v2.x, y, v1.y, 1.0);
        self.flat_vertex(v1.x, y, v1.y, 1.0);

        self.flat_vertex(v1.x, y, v2.y, 1.0);
        self.flat_vertex(v2.x, y, v2.y, 1.0);
        self.flat_vertex(v2.x, y, v1.y, 1.0);

        for i in range(0, points.len()) {
            let (v1, v2) = (points[i], points[(i + 1) % points.len()]);
            let t = 3.0;
            let e = (v1 - v2).normalized() * 0.02;
            let n = e.normal();
            let (v1, v2) = (v1 - e, v2 + e);

            self.flat_vertex(v2.x + n.x*t, y, v2.y + n.y*t, 1.0);
            self.flat_vertex(v1.x + n.x*t, y, v1.y + n.y*t, 1.0);
            self.flat_vertex(v1.x + n.x, y, v1.y + n.y, 1.0);

            self.flat_vertex(v1.x + n.x, y, v1.y + n.y, 1.0);
            self.flat_vertex(v2.x + n.x, y, v2.y + n.y, 1.0);
            self.flat_vertex(v2.x + n.x*t, y, v2.y + n.y*t, 1.0);
        }
    }

    fn convex_flat(&mut self, sector: &WadSector, points: &[Vec2f]) {
        let floor = from_wad_height(sector.floor_height);
        let ceiling = from_wad_height(sector.ceiling_height);
        let v0 = points[0];
        for i in range(1, points.len()) {
            let (v1, v2) = (points[i], points[(i + 1) % points.len()]);
            self.flat_vertex(v0.x, floor, v0.y, 1.0);
            self.flat_vertex(v1.x, floor, v1.y, 1.0);
            self.flat_vertex(v2.x, floor, v2.y, 1.0);

            self.flat_vertex(v2.x, ceiling, v2.y, -1.0);
            self.flat_vertex(v1.x, ceiling, v1.y, -1.0);
            self.flat_vertex(v0.x, ceiling, v0.y, -1.0);
        }
    }

    fn push_seg(&mut self, seg: &WadSeg, (low, high): (WadCoord, WadCoord)) {
        if !DRAW_WALLS { return; }
        let (v1, v2) = (self.wad.vertex(seg.start_vertex),
                        self.wad.vertex(seg.end_vertex));
        let normal = (v2 - v1).normalized().normal();
        let (low, high) = (from_wad_height(low), from_wad_height(high));

        self.wall_vertex(&v1, low,  &normal);
        self.wall_vertex(&v2, low,  &normal);
        self.wall_vertex(&v1, high, &normal);

        self.wall_vertex(&v2, low,  &normal);
        self.wall_vertex(&v2, high, &normal);
        self.wall_vertex(&v1, high, &normal);
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
