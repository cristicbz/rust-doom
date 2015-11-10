use contact::ContactInfo;
use num::Zero;
use vector::{Vec3f, Vec2f, Vector};

pub struct Sphere {
    pub center: Vec3f,
    pub radius: f32,
}

impl Sphere {
    pub fn new(center: Vec3f, radius: f32) -> Sphere {
        Sphere {
            center: center,
            radius: radius,
        }
    }

    pub fn sweep_triangle(&self,
                          triangle: &[Vec3f; 3],
                          normal: &Vec3f,
                          vel: &Vec3f)
                          -> Option<ContactInfo> {
        let Sphere { ref center, radius } = *self;
        let speed = vel.norm();
        if speed == 0.0 {
            return None;
        }
        let nvel = *vel / speed;
        let normal_dot_nvel = normal.dot(&nvel);

        if normal_dot_nvel >= 0.0 {
            return None;
        }

        let mut contact_normal = Vec3f::zero();
        let mut collision = false;
        let mut min_distance = 1e4;
        let intercept = -normal.dot(&triangle[0]);

        // Sphere against plane.
        let signed_plane_distance = normal.dot(center) + intercept;
        if signed_plane_distance < -radius {
            return None;
        }

        if signed_plane_distance >= radius {
            let distance = -(signed_plane_distance - radius) / normal_dot_nvel;
            let on_plane = *center + nvel * distance;
            if is_point_inside_triangle(triangle, &on_plane) {
                min_distance = distance;
                contact_normal = *normal;
                collision = true;
            }
        }

        // Sphere against vertices.
        for vertex in triangle {
            if let Some(d) = intersect_sphere_line(center, radius, vertex, &(*vertex - nvel)) {
                if d >= 0.0 && d < min_distance {
                    min_distance = d;
                    contact_normal = *center - (*vertex - nvel * d);
                    collision = true;
                }
            }
        }

        // Sphere against edges.
        for i in 0..3 {
            let e1 = triangle[i];
            let e2 = triangle[(i + 1) % 3];
            let edge = e2 - e1;
            let edge_normal = nvel.cross(edge).normalized();
            let edge_intercept = -edge_normal.dot(&e1);
            let edge_distance = edge_normal.dot(center) + edge_intercept;
            if edge_distance.abs() > radius {
                continue;
            }

            let circle_radius = (radius * radius - edge_distance * edge_distance).sqrt();
            let circle_center = *center - edge_normal * edge_distance;
            let e1_to_circle_center = circle_center - e1;
            let disp = edge * (e1_to_circle_center.dot(&edge) / edge.squared_norm());
            let on_line = e1 + disp;
            let circle_center_to_on_line = (on_line - circle_center).normalized();
            let candidate = circle_center_to_on_line * circle_radius + circle_center;

            let edge_normal_abs = Vec3f::new(edge_normal[0].abs(),
                                             edge_normal[1].abs(),
                                             edge_normal[2].abs());
            let (dim1, dim2) = if edge_normal_abs[0] > edge_normal_abs[1] &&
                                  edge_normal_abs[0] > edge_normal_abs[2] {
                (1, 2)
            } else if edge_normal_abs[1] > edge_normal_abs[2] {
                (0, 2)
            } else {
                (0, 1)
            };

            let candidate_plus_nvel = candidate + nvel;
            let t = match intersect_line_line(&Vec2f::new(candidate[dim1], candidate[dim2]),
                                              &Vec2f::new(candidate_plus_nvel[dim1],
                                                          candidate_plus_nvel[dim2]),
                                              &Vec2f::new(e1[dim1], e1[dim2]),
                                              &Vec2f::new(e2[dim1], e2[dim2])) {
                Some(distance) if distance >= 0.0 && distance < min_distance => distance,
                _ => continue,
            };
            let intersection = candidate + nvel * t;
            if (e1 - intersection).dot(&(e2 - intersection)) > 0.0 {
                continue;
            }
            min_distance = t;
            contact_normal = *center - candidate;
            collision = true;
        }

        if collision {
            Some(ContactInfo {
                normal: contact_normal.normalized(),
                time: min_distance / speed,
            })
        } else {
            None
        }
    }
}


fn intersect_sphere_line(center: &Vec3f, radius: f32, p1: &Vec3f, p2: &Vec3f) -> Option<f32> {
    let edge = *p2 - *p1;
    let a = edge.squared_norm();
    let b = 2.0 * edge.dot(&(*p1 - *center));
    let c = center.squared_norm() + p1.squared_norm() - 2.0 * center.dot(p1) - radius * radius;
    lowest_quadratic_root(a, b, c)
}

fn lowest_quadratic_root(a: f32, b: f32, c: f32) -> Option<f32> {
    let i = b * b - 4.0 * a * c;
    if i < 0.0 {
        None
    } else {
        let i = i.sqrt();
        let a2 = 2.0 * a;
        let i1 = (-b + i) / a2;
        let i2 = (-b - i) / a2;
        if i1 < i2 {
            Some(i1)
        } else {
            Some(i2)
        }
    }
}

fn intersect_line_line(p1: &Vec2f, p2: &Vec2f, p3: &Vec2f, p4: &Vec2f) -> Option<f32> {
    let d1 = *p2 - *p1;
    let d2 = *p3 - *p4;
    let denom = d2[1] * d1[0] - d2[0] * d1[1];
    if denom == 0.0 {
        None
    } else {
        let dist = d2[0] * (p1[1] - p3[1]) - d2[1] * (p1[0] - p3[0]);
        Some(dist / denom)
    }
}

fn is_point_inside_triangle(verts: &[Vec3f; 3], point: &Vec3f) -> bool {
    let u = verts[1] - verts[0];
    let v = verts[2] - verts[0];
    let n = u.cross(v);
    let w = *point - verts[0];
    let n2 = n.squared_norm();

    let gamma = u.cross(w).dot(&n) / n2;
    let beta = w.cross(v).dot(&n) / n2;
    let alpha = 1.0 - gamma - beta;

    let a = 0.0;
    let b = 1.0 - a;

    a <= alpha && alpha <= b && a <= gamma && gamma <= b && a <= beta && beta <= b
}
