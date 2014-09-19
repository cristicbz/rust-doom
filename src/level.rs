use check_gl;
use gl;
use numvec::{Vec2f, Vec2, Numvec};
use shader::{Shader, Uniform};
use mat4::Mat4;
use std::vec::Vec;
use std::ptr;
use vbo::VertexBuffer;
use wad;
use wad::util::{from_wad_height, from_wad_coords, no_lower_texture,
                no_middle_texture, no_upper_texture, parse_child_id};
use wad::types::*;


static DRAW_WALLS : bool = true;
static WIRE_FLOORS : bool = false;
static SINGLE_SEGMENT : i16 = -1;
static SSECTOR_BSP_TOLERANCE : f32 = 1e-4;
static SSECTOR_SEG_TOLERANCE : f32 = 0.1;


pub struct Level {
    start_pos: Vec2f,
    mvp_uniform: Uniform,
    shader: Shader,
    vbo: VertexBuffer,
}


impl Level {
    pub fn new(wad: &mut wad::Archive, name: &LevelName) -> Level {
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
            shader: shader,
            vbo: wad_to_vbo(&data)
        }
    }

    pub fn get_start_pos<'a>(&'a self) -> &'a Vec2f { &self.start_pos }

    pub fn render(&self, projection_view: &Mat4) {
        //check_gl!(gl::Enable(gl::CULL_FACE));
        self.shader.bind();
        self.shader.set_uniform_mat4(self.mvp_uniform, projection_view);
        check_gl!(gl::EnableVertexAttribArray(0));
        self.vbo.bind();
        check_gl_unsafe!(gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE,
                                                 0, ptr::null()));
        check_gl!(gl::DrawArrays(gl::TRIANGLES, 0,
                                 (self.vbo.len() / 3) as i32));
        self.vbo.unbind();
        check_gl!(gl::DisableVertexAttribArray(0));
    }
}

struct Line {
    origin: Vec2f,
    displace: Vec2f,
}

impl Line {
    pub fn from_origin_and_displace(origin: Vec2f, displace: Vec2f) -> Line {
        Line { origin: origin, displace: displace.normalized() }
    }

    pub fn from_points(origin: Vec2f, towards: &Vec2f) -> Line {
        Line { origin: origin, displace: (towards - origin).normalized() }
    }

    pub fn inverted_halfspaces(&self) -> Line {
        Line { origin: self.origin, displace: -self.displace }
    }

    pub fn signed_distance(&self, to: &Vec2f) -> f32 {
        to.cross(&self.displace) + self.displace.cross(&self.origin)
    }

    pub fn intersect_offset(&self, other: &Line) -> Option<f32> {
        let numerator = self.displace.cross(&other.displace);
        if numerator.abs() < 1e-10 {
            None
        } else {
            Some((other.origin - self.origin).cross(&other.displace) /
                 numerator)
        }
    }

    pub fn at_offset(&self, offset: f32) -> Vec2f {
        self.origin + self.displace * offset
    }
}


fn vbo_push_wall(vbo_data: &mut Vec<f32>, lvl: &wad::Level,
                 seg: &WadSeg, (low, high): (WadCoord, WadCoord)) {
    if !DRAW_WALLS { return; }
    let (v1, v2) = (lvl.vertex(seg.start_vertex), lvl.vertex(seg.end_vertex));
    let (low, high) = (from_wad_height(low), from_wad_height(high));
    vbo_data.push(v1.x); vbo_data.push(low); vbo_data.push(v1.y);
    vbo_data.push(v2.x); vbo_data.push(low); vbo_data.push(v2.y);
    vbo_data.push(v1.x); vbo_data.push(high); vbo_data.push(v1.y);
    vbo_data.push(v2.x); vbo_data.push(low); vbo_data.push(v2.y);
    vbo_data.push(v2.x); vbo_data.push(high); vbo_data.push(v2.y);
    vbo_data.push(v1.x); vbo_data.push(high); vbo_data.push(v1.y);
}


fn ssector_to_vbo(lvl: &wad::Level, vbo: &mut Vec<f32>, lines: &mut Vec<Line>,
                  ssector: &WadSubsector) {
    let segs = lvl.ssector_segs(ssector);

    let mut points : Vec<Vec2f> = Vec::new();
    for l1 in lines.iter() {
        for l2 in lines.iter() {
            let point = l1.at_offset(
                match l1.intersect_offset(l2) {
                    Some(offset) => offset,
                    None => continue
                });

            let mut push = true;
            for l3 in lines.iter() {
                if l3.signed_distance(&point) < -SSECTOR_BSP_TOLERANCE {
                    push = false;
                    break;
                }
            }

            if push {
                for seg in segs.iter() {
                    let (v1, v2) = lvl.seg_vertices(seg);
                    if Line::from_points(v1, &v2)
                            .signed_distance(&point) > SSECTOR_SEG_TOLERANCE {
                        push = false;
                        break;
                    }
                }
                if push {
                    points.push(point);
                }
            }
        }
    }
    for seg in segs.iter() {
        let (v1, v2) = lvl.seg_vertices(seg);
        points.push(v1);
        points.push(v2);
    }
    points.sort_by(
        |a, b| {
            if a.x < b.x {
                Less
            } else if a.x > b.x {
                Greater
            } else if a.y < b.y {
                Less
            } else if a.y > b.y {
                Greater
            } else {
                Equal
            }
        });

    let mut k = 1;
    for i in range(1, points.len()) {
        let d = points[i] - points[i - 1];
        if d.x.abs() > 1e-10 || d.y.abs() > 1e-10 {
            *points.get_mut(k) = points[i];
            k = k + 1;
        }
    }
    points.truncate(k);
    let mut center = Vec2::zero();
    for p in points.iter() {
        center = center + *p;
    }
    center = center / (points.len() as f32);
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

            let d = ac.cross(&bc);

            if d < 0.0 { Less }
            else if d > 0.0 { Greater }
            else if ac.squared_norm() > bc.squared_norm() { Greater }
            else { Less }

        });

    let seg = &segs[0];
    let line = lvl.seg_linedef(seg);
    let sector = lvl.sidedef_sector(
        if seg.direction == 0 {
           lvl.right_sidedef(line)
        } else {
           lvl.left_sidedef(line)
        });
    let floor = from_wad_height(sector.floor_height);
    let ceil = from_wad_height(sector.ceiling_height);

    if WIRE_FLOORS {
        for p in [center].iter() {
            let v1 = p - Vec2::new(0.1, 0.1);
            let v2 = p + Vec2::new(0.1, 0.1);
            vbo.push(v1.x); vbo.push(floor); vbo.push(v1.y);
            vbo.push(v2.x); vbo.push(floor); vbo.push(v1.y);
            vbo.push(v1.x); vbo.push(floor); vbo.push(v2.y);
            vbo.push(v2.x); vbo.push(floor); vbo.push(v1.y);
            vbo.push(v2.x); vbo.push(floor); vbo.push(v2.y);
            vbo.push(v1.x); vbo.push(floor); vbo.push(v2.y);
        }
    }

    let v0 = center;
    for i in range(0, points.len()) {
        let (v1, v2) = (points[i % points.len()], points[(i + 1) % points.len()]);
        if WIRE_FLOORS {
            let n = (v1 - v2).normal().normalized() * 0.03;

            vbo.push(v1.x + n.x); vbo.push(floor); vbo.push(v1.y + n.y);
            vbo.push(v1.x + n.x*2.0); vbo.push(floor); vbo.push(v1.y + n.y*2.0);
            vbo.push(v2.x + n.x*2.0); vbo.push(floor); vbo.push(v2.y + n.y*2.0);

            vbo.push(v1.x + n.x); vbo.push(floor); vbo.push(v1.y + n.y);
            vbo.push(v2.x + n.x); vbo.push(floor); vbo.push(v2.y + n.y);
            vbo.push(v2.x + n.x*2.0); vbo.push(floor); vbo.push(v2.y + n.y*2.0);
        } else {
            vbo.push(v0.x); vbo.push(floor); vbo.push(v0.y);
            vbo.push(v1.x); vbo.push(floor); vbo.push(v1.y);
            vbo.push(v2.x); vbo.push(floor); vbo.push(v2.y);

            vbo.push(v0.x); vbo.push(ceil); vbo.push(v0.y);
            vbo.push(v1.x); vbo.push(ceil); vbo.push(v1.y);
            vbo.push(v2.x); vbo.push(ceil); vbo.push(v2.y);
        }
    }
}



fn node_to_vbo(lvl: &wad::Level, vbo: &mut Vec<f32>, lines: &mut Vec<Line>,
               node: &WadNode) {
    let (left, leaf_left) = parse_child_id(node.left);
    let (right, leaf_right) = parse_child_id(node.right);
    let partition = Line::from_origin_and_displace(
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
            if !no_middle_texture(side) {
                vbo_push_wall(&mut vbo, lvl, seg,
                              (sector.floor_height, sector.ceiling_height));
            }
        } else if linedef.right_side == -1 {
            let side = lvl.left_sidedef(linedef);
            let sector = lvl.sidedef_sector(side);
            if !no_middle_texture(side) {
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
                if !no_lower_texture(lside) {
                    vbo_push_wall(&mut vbo, lvl, seg, (lfloor, rfloor))
                }
            } else if lfloor > rfloor {
                if !no_lower_texture(rside) {
                    vbo_push_wall(&mut vbo, lvl, seg, (rfloor, lfloor))
                }
            }

            if lceil < rceil {
                if !no_upper_texture(rside) {
                    vbo_push_wall(&mut vbo, lvl, seg, (lceil, rceil))
                }
            } else if lceil > rceil {
                if !no_upper_texture(lside) {
                    vbo_push_wall(&mut vbo, lvl, seg, (rceil, lceil))
                }
            }

            if !no_middle_texture(lside) {
                vbo_push_wall(&mut vbo, lvl, seg, (lfloor, lceil));
            }
            if !no_middle_texture(rside) {
                vbo_push_wall(&mut vbo, lvl, seg, (rfloor, rceil));
            }
        }
    }
    node_to_vbo(lvl, &mut vbo, &mut Vec::new(), lvl.nodes.last().unwrap());
    VertexBuffer::new_with_data(gl::ARRAY_BUFFER, gl::STATIC_DRAW,
                                vbo.as_slice())
}


