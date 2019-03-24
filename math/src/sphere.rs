use super::contact::ContactInfo;
use super::prelude::*;
use super::{vec2, vec3, Pnt3f, Vec2f, Vec3f};

#[derive(Copy, Clone, Debug)]
pub struct Sphere {
    pub center: Pnt3f,
    pub radius: f32,
}

impl Sphere {
    pub fn new(center: Pnt3f, radius: f32) -> Self {
        Self { center, radius }
    }

    pub fn sweep_triangle(
        &self,
        triangle: &[Pnt3f; 3],
        normal: Vec3f,
        vel: Vec3f,
    ) -> Option<ContactInfo> {
        let Self { center, radius } = *self;
        let speed = vel.magnitude();
        if speed == 0.0 {
            return None;
        }
        let nvel = vel / speed;
        let normal_dot_nvel = normal.dot(nvel);

        if normal_dot_nvel >= 0.0 {
            return None;
        }

        let mut contact_normal = Vec3f::zero();
        let mut collision = false;
        let mut min_distance = 1e4;
        let intercept = -triangle[0].dot(normal);

        // Sphere against plane.
        let signed_plane_distance = center.dot(normal) + intercept;
        if signed_plane_distance < -radius {
            return None;
        }

        if signed_plane_distance >= radius {
            let distance = -(signed_plane_distance - radius) / normal_dot_nvel;
            let on_plane = center + nvel * distance;
            if is_point_inside_triangle(triangle, on_plane) {
                min_distance = distance;
                contact_normal = normal;
                collision = true;
            }
        }

        // Sphere against vertices.
        for &vertex in triangle {
            if let Some(d) = intersect_sphere_line(center, radius, vertex, vertex + (-nvel)) {
                if d >= 0.0 && d < min_distance {
                    min_distance = d;
                    contact_normal = center - (vertex + nvel * (-d));
                    collision = true;
                }
            }
        }

        // Sphere against edges.
        for (&e1, &e2) in triangle
            .iter()
            .zip(triangle.iter().skip(1).chain(Some(&triangle[0])))
        {
            let edge = e2 - e1;
            let edge_normal = nvel.cross(edge).normalize_or_zero();
            let edge_intercept = -e1.dot(edge_normal);
            let edge_distance = center.dot(edge_normal) + edge_intercept;
            if edge_distance.abs() > radius {
                continue;
            }

            let circle_radius = (radius * radius - edge_distance * edge_distance).sqrt();
            let circle_center = center + edge_normal * (-edge_distance);
            let e1_to_circle_center = circle_center - e1;
            let disp = edge * (e1_to_circle_center.dot(edge) / edge.magnitude2());
            let on_line = e1 + disp;
            let circle_center_to_on_line = (on_line - circle_center).normalize_or_zero();
            let candidate = circle_center + circle_center_to_on_line * circle_radius;

            let edge_normal_abs = vec3(
                edge_normal[0].abs(),
                edge_normal[1].abs(),
                edge_normal[2].abs(),
            );
            let (dim1, dim2) = if edge_normal_abs[0] > edge_normal_abs[1]
                && edge_normal_abs[0] > edge_normal_abs[2]
            {
                (1, 2)
            } else if edge_normal_abs[1] > edge_normal_abs[2] {
                (0, 2)
            } else {
                (0, 1)
            };

            let candidate_plus_nvel = candidate + nvel;
            let t = match intersect_line_line(
                vec2(candidate[dim1], candidate[dim2]),
                vec2(candidate_plus_nvel[dim1], candidate_plus_nvel[dim2]),
                vec2(e1[dim1], e1[dim2]),
                vec2(e2[dim1], e2[dim2]),
            ) {
                Some(distance) if distance >= 0.0 && distance < min_distance => distance,
                _ => continue,
            };
            let intersection = candidate + nvel * t;
            if (e1 - intersection).dot(e2 - intersection) > 0.0 {
                continue;
            }
            min_distance = t;
            contact_normal = center - candidate;
            collision = true;
        }

        if collision {
            Some(ContactInfo {
                normal: contact_normal.normalize_or_zero(),
                time: min_distance / speed,
            })
        } else {
            None
        }
    }
}

fn intersect_sphere_line(center: Pnt3f, radius: f32, p1: Pnt3f, p2: Pnt3f) -> Option<f32> {
    let edge = p2 - p1;
    let a = edge.magnitude2();
    let b = 2.0 * edge.dot(p1 - center);
    let c = center.to_vec().magnitude2() + p1.to_vec().magnitude2()
        - 2.0 * center.dot(p1.to_vec())
        - radius * radius;
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

fn intersect_line_line(p1: Vec2f, p2: Vec2f, p3: Vec2f, p4: Vec2f) -> Option<f32> {
    let d1 = p2 - p1;
    let d2 = p3 - p4;
    let denom = d2[1] * d1[0] - d2[0] * d1[1];
    if denom == 0.0 {
        None
    } else {
        let dist = d2[0] * (p1[1] - p3[1]) - d2[1] * (p1[0] - p3[0]);
        Some(dist / denom)
    }
}

fn is_point_inside_triangle(verts: &[Pnt3f; 3], point: Pnt3f) -> bool {
    let u = verts[1] - verts[0];
    let v = verts[2] - verts[0];
    let n = u.cross(v);
    let w = point - verts[0];
    let n2 = n.magnitude2();

    let gamma = u.cross(w).dot(n) / n2;
    let beta = w.cross(v).dot(n) / n2;
    let alpha = 1.0 - gamma - beta;

    0.0 <= alpha && alpha <= 1.0 && 0.0 <= gamma && gamma <= 1.0 && 0.0 <= beta && beta <= 1.0
}
