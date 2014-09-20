use check_gl;
use gl;
use line::{Line2, Line2f};
use numvec::{Vec2f, Vec2, Vec3f, Numvec};
use shader::{Shader, Uniform};
use mat4::Mat4;
use std::vec::Vec;
use std::ptr;
use vbo::VertexBuffer;
use wad;
use wad::util::{from_wad_height, from_wad_coords, is_untextured, is_sky_texture,
                parse_child_id};
use wad::types::*;
use libc::c_void;


static DRAW_WALLS : bool = true;
static WIRE_FLOORS : bool = false;
static ZERO_FLOORS : bool = false;
static SINGLE_SEGMENT : i16 = -1;
static BSP_TOLERANCE : f32 = 1e-3;
static SEG_TOLERANCE : f32 = 0.1;
static RENDER_SKY : bool = true;

fn should_render(name: &WadName) -> bool {
    !is_untextured(name) && (RENDER_SKY | !is_sky_texture(name))
}


pub struct Level {
    start_pos: Vec2f,
    mvp_uniform: Uniform,
    eye_uniform: Uniform,
    shader: Shader,
    vbo: VertexBuffer,
}


impl Level {
    pub fn new(wad: &mut wad::Archive, name: &WadName) -> Level {
        let data = wad::Level::from_archive(wad, name);

        let mut start_pos = Vec2::zero();
        for thing in data.things.iter() {
            if thing.thing_type == 1 {  // Player 1 start position.
                start_pos = from_wad_coords(thing.x, thing.y);
                info!("Player start position: {}.", start_pos);
            }
        }

        let shader = Shader::new_from_files(
            &Path::new("src/shaders/basic.vertex.glsl"),
            &Path::new("src/shaders/basic.fragment.glsl")).unwrap();

        Level {
            start_pos: start_pos,
            mvp_uniform: shader.get_uniform("mvp_transform").unwrap(),
            eye_uniform: shader.get_uniform("eye_uniform").unwrap(),
            shader: shader,
            vbo: wad_to_vbo(&data)
        }
    }

    pub fn get_start_pos<'a>(&'a self) -> &'a Vec2f { &self.start_pos }

    pub fn render(&self, projection_view: &Mat4, eye: &Vec3f) {
        check_gl!(gl::Enable(gl::CULL_FACE));
        self.shader.bind();
        self.shader.set_uniform_mat4(self.mvp_uniform, projection_view);
        self.shader.set_uniform_vec3f(self.eye_uniform, eye);
        check_gl!(gl::EnableVertexAttribArray(0));
        check_gl!(gl::EnableVertexAttribArray(1));
        self.vbo.bind();
        check_gl_unsafe!(gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE,
                                                 24, ptr::null()));
        check_gl_unsafe!(gl::VertexAttribPointer(1, 3, gl::FLOAT, gl::FALSE,
                                                 24, 12 as *const c_void));
        check_gl!(gl::DrawArrays(gl::TRIANGLES, 0,
                                 (self.vbo.len() / 6) as i32));
        self.vbo.unbind();
        check_gl!(gl::DisableVertexAttribArray(0));
        check_gl!(gl::DisableVertexAttribArray(1));
    }
}


fn vbo_push_wall(vbo_data: &mut Vec<f32>, lvl: &wad::Level,
                 seg: &WadSeg, (low, high): (WadCoord, WadCoord)) {
    if !DRAW_WALLS { return; }
    let (v1, v2) = (lvl.vertex(seg.start_vertex), lvl.vertex(seg.end_vertex));
    let edge = (v2 - v1).normalized();
    let (low, high) = (from_wad_height(low), from_wad_height(high));
    vbo_data.push(v1.x); vbo_data.push(low); vbo_data.push(v1.y);
    vbo_data.push(-edge.y); vbo_data.push(0.0); vbo_data.push(edge.x);

    vbo_data.push(v2.x); vbo_data.push(low); vbo_data.push(v2.y);
    vbo_data.push(-edge.y); vbo_data.push(0.0); vbo_data.push(edge.x);

    vbo_data.push(v1.x); vbo_data.push(high); vbo_data.push(v1.y);
    vbo_data.push(-edge.y); vbo_data.push(0.0); vbo_data.push(edge.x);

    vbo_data.push(v2.x); vbo_data.push(low); vbo_data.push(v2.y);
    vbo_data.push(-edge.y); vbo_data.push(0.0); vbo_data.push(edge.x);

    vbo_data.push(v2.x); vbo_data.push(high); vbo_data.push(v2.y);
    vbo_data.push(-edge.y); vbo_data.push(0.0); vbo_data.push(edge.x);

    vbo_data.push(v1.x); vbo_data.push(high); vbo_data.push(v1.y);
    vbo_data.push(-edge.y); vbo_data.push(0.0); vbo_data.push(edge.x);
}


fn ssector_to_vbo(lvl: &wad::Level, vbo: &mut Vec<f32>, lines: &mut Vec<Line2f>,
                  ssector: &WadSubsector) {
    let segs = lvl.ssector_segs(ssector);

    // Add all points that are part of this subsector. Duplicates are removed
    // later.
    let mut points : Vec<Vec2f> = Vec::new();

    // The bounds of the segs form the 'explicit' points.
    for seg in segs.iter() {
        let (v1, v2) = lvl.seg_vertices(seg);
        points.push(v1);
        points.push(v2);
    }

    // The convex polyon defined at the intersection of the partition lines,
    // intersected with the half-volume of the segs form the 'implicit' points.
    for i_line in range(0, lines.len() - 1) {
        for j_line in range(i_line + 1, lines.len()) {
            let (l1, l2) = (&(*lines)[i_line], &(*lines)[j_line]);
            let point = match l1.intersect_point(l2) {
                Some(p) => p,
                None => continue
            };

            let line_dist = |l : &Line2f| l.signed_distance(&point);
            let seg_dist = |s : &WadSeg|
                Line2::from_point_pair(lvl.seg_vertices(s))
                    .signed_distance(&point);

            // The intersection point must lie both within the BSP volume and
            // the segs volume.
            if lines.iter().map(line_dist).all(|d| d >= -BSP_TOLERANCE) &&
               segs.iter().map(seg_dist).all(|d| d <= SEG_TOLERANCE) {
                points.push(point);
            }
        }
    }

    // Sort points in polygonal order around their center.
    let mut center = Vec2::zero();
    for p in points.iter() { center = center + *p; }
    let center = center / (points.len() as f32);
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
        let d = points[i_point] - points[i_point - 1];
        if d.x.abs() > 1e-10 || d.y.abs() > 1e-10 {
            *points.get_mut(n_unique) = points[i_point];
            n_unique = n_unique + 1;
        }
    }
    points.truncate(n_unique);

    let seg = &segs[0];
    let line = lvl.seg_linedef(seg);
    let sector = lvl.sidedef_sector(
        if seg.direction == 0 {
           lvl.right_sidedef(line)
        } else {
           lvl.left_sidedef(line)
        });
    let floor = if ZERO_FLOORS {
        0.0
    } else {
        from_wad_height(sector.floor_height)
    };
    let ceil = from_wad_height(sector.ceiling_height);

    if WIRE_FLOORS {
        for p in [center].iter() {
            let v1 = p - Vec2::new(0.03, 0.03);
            let v2 = p + Vec2::new(0.03, 0.03);

            vbo.push(v1.x); vbo.push(floor); vbo.push(v2.y);
            vbo.push(0.0); vbo.push(1.0); vbo.push(0.0);

            vbo.push(v2.x); vbo.push(floor); vbo.push(v1.y);
            vbo.push(0.0); vbo.push(1.0); vbo.push(0.0);

            vbo.push(v1.x); vbo.push(floor); vbo.push(v1.y);
            vbo.push(0.0); vbo.push(1.0); vbo.push(0.0);

            vbo.push(v1.x); vbo.push(floor); vbo.push(v2.y);
            vbo.push(0.0); vbo.push(1.0); vbo.push(0.0);

            vbo.push(v2.x); vbo.push(floor); vbo.push(v2.y);
            vbo.push(0.0); vbo.push(1.0); vbo.push(0.0);

            vbo.push(v2.x); vbo.push(floor); vbo.push(v1.y);
            vbo.push(0.0); vbo.push(1.0); vbo.push(0.0);
        }
    }

    let v0 = center;
    for i in range(0, points.len()) {
        let (v1, v2) = (points[i % points.len()], points[(i + 1) % points.len()]);
        if WIRE_FLOORS {
            let t = 3.0;
            let e = (v1 - v2).normalized() * 0.02;
            let n = e.normal();
            let (v1, v2) = (v1 - e, v2 + e);

            vbo.push(v2.x + n.x*t); vbo.push(floor); vbo.push(v2.y + n.y*t);
            vbo.push(0.0); vbo.push(1.0); vbo.push(0.0);
            vbo.push(v1.x + n.x*t); vbo.push(floor); vbo.push(v1.y + n.y*t);
            vbo.push(0.0); vbo.push(1.0); vbo.push(0.0);
            vbo.push(v1.x + n.x); vbo.push(floor); vbo.push(v1.y + n.y);
            vbo.push(0.0); vbo.push(1.0); vbo.push(0.0);

            vbo.push(v1.x + n.x); vbo.push(floor); vbo.push(v1.y + n.y);
            vbo.push(0.0); vbo.push(1.0); vbo.push(0.0);
            vbo.push(v2.x + n.x); vbo.push(floor); vbo.push(v2.y + n.y);
            vbo.push(0.0); vbo.push(1.0); vbo.push(0.0);
            vbo.push(v2.x + n.x*t); vbo.push(floor); vbo.push(v2.y + n.y*t);
            vbo.push(0.0); vbo.push(1.0); vbo.push(0.0);
        } else {
            vbo.push(v0.x); vbo.push(floor); vbo.push(v0.y);
            vbo.push(0.0); vbo.push(1.0); vbo.push(0.0);

            vbo.push(v1.x); vbo.push(floor); vbo.push(v1.y);
            vbo.push(0.0); vbo.push(1.0); vbo.push(0.0);

            vbo.push(v2.x); vbo.push(floor); vbo.push(v2.y);
            vbo.push(0.0); vbo.push(1.0); vbo.push(0.0);

            vbo.push(v2.x); vbo.push(ceil); vbo.push(v2.y);
            vbo.push(0.0); vbo.push(-1.0); vbo.push(0.0);

            vbo.push(v1.x); vbo.push(ceil); vbo.push(v1.y);
            vbo.push(0.0); vbo.push(-1.0); vbo.push(0.0);

            vbo.push(v0.x); vbo.push(ceil); vbo.push(v0.y);
            vbo.push(0.0); vbo.push(-1.0); vbo.push(0.0);
        }
    }
}



fn node_to_vbo(lvl: &wad::Level, vbo: &mut Vec<f32>, lines: &mut Vec<Line2f>,
               node: &WadNode) {
    let (left, leaf_left) = parse_child_id(node.left);
    let (right, leaf_right) = parse_child_id(node.right);
    let partition = Line2::from_origin_and_displace(
        from_wad_coords(node.line_x, node.line_y),
        from_wad_coords(node.step_x, node.step_y));

    lines.push(partition);
    if leaf_left {
        if left == SINGLE_SEGMENT as uint || SINGLE_SEGMENT == -1 {
            ssector_to_vbo(lvl, vbo, lines, &lvl.subsectors[left]);
        }
    } else {
        node_to_vbo(lvl, vbo, lines, &lvl.nodes[left]);
    }
    lines.pop();


    lines.push(partition.inverted_halfspaces());
    if leaf_right {
        if right == SINGLE_SEGMENT as uint || SINGLE_SEGMENT == -1 {
            ssector_to_vbo(lvl, vbo, lines, &lvl.subsectors[right]);
        }
    } else {
        node_to_vbo(lvl, vbo, lines, &lvl.nodes[right]);
    }
    lines.pop();
}


fn wad_to_vbo(lvl: &wad::Level) -> VertexBuffer {
    let mut vbo: Vec<f32> = Vec::with_capacity(lvl.linedefs.len() * 18);
    for seg in lvl.segs.iter() {
        let linedef = lvl.seg_linedef(seg);
        if linedef.left_side == -1 {
            let side = lvl.right_sidedef(linedef);
            let sector = lvl.sidedef_sector(side);
            if should_render(&side.middle_texture) {
                vbo_push_wall(&mut vbo, lvl, seg,
                              (sector.floor_height, sector.ceiling_height));
            }
        } else if linedef.right_side == -1 {
            let side = lvl.left_sidedef(linedef);
            let sector = lvl.sidedef_sector(side);
            if should_render(&side.middle_texture) {
                vbo_push_wall(&mut vbo, lvl, seg,
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
                    vbo_push_wall(&mut vbo, lvl, seg, (lfloor, rfloor))
                }
            } else if lfloor > rfloor {
                if should_render(&rside.lower_texture) {
                    vbo_push_wall(&mut vbo, lvl, seg, (rfloor, lfloor))
                }
            }

            if lceil < rceil {
                if should_render(&rside.upper_texture) {
                    vbo_push_wall(&mut vbo, lvl, seg, (lceil, rceil))
                }
            } else if lceil > rceil {
                if should_render(&lside.upper_texture) {
                    vbo_push_wall(&mut vbo, lvl, seg, (rceil, lceil))
                }
            }

            if should_render(&lside.middle_texture) {
                vbo_push_wall(&mut vbo, lvl, seg, (lfloor, lceil));
            }
            if should_render(&rside.middle_texture) {
                vbo_push_wall(&mut vbo, lvl, seg, (rfloor, rceil));
            }
        }
    }
    node_to_vbo(lvl, &mut vbo, &mut Vec::new(), lvl.nodes.last().unwrap());
    VertexBuffer::new_with_data(gl::ARRAY_BUFFER, gl::STATIC_DRAW,
                                vbo.as_slice())
}


