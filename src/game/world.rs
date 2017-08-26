use math::{ContactInfo, Line2f, Sphere, Vec2f, Vec3f, Vector};
use num::Zero;
use std::{f32, i32};
use std::cell::RefCell;
use wad::{Branch, LevelVisitor, SkyPoly, SkyQuad, StaticPoly, StaticQuad};


pub struct World {
    nodes: Vec<Node>,
    chunks: Vec<Chunk>,
    triangles: Vec<Triangle>,
    verts: Vec<Vec3f>,

    node_stack: RefCell<Vec<usize>>,
}

impl World {
    pub fn new() -> World {
        World {
            nodes: Vec::with_capacity(128),
            chunks: Vec::with_capacity(128),
            triangles: Vec::with_capacity(1024),
            verts: Vec::with_capacity(4096),
            node_stack: RefCell::new(Vec::with_capacity(32)),
        }
    }

    pub fn sweep_sphere(&self, sphere: &Sphere, vel: &Vec3f) -> Option<ContactInfo> {
        let mut stack = self.node_stack.borrow_mut();
        stack.push(0);
        let mut first_contact = ContactInfo {
            time: f32::INFINITY,
            normal: Vec3f::zero(),
        };

        while let Some(index) = stack.pop() {
            for child in self.nodes[index].intersect_sphere(sphere, vel) {
                let chunk = match child {
                    Child::Stump => continue,
                    Child::Node(index) => {
                        stack.push(index);
                        continue;
                    }
                    Child::Leaf(index) => &self.chunks[index],
                };
                let tris = &self.triangles[chunk.tri_start as usize..chunk.tri_end as usize];
                first_contact = tris.iter()
                    .filter_map(|tri| self.sweep_sphere_triangle(sphere, vel, tri))
                    .fold(
                        first_contact,
                        |first, current| if first.time < current.time {
                            first
                        } else {
                            current
                        },
                    );
            }
        }
        if first_contact.time < f32::INFINITY {
            Some(first_contact)
        } else {
            None
        }
    }

    fn sweep_sphere_triangle(
        &self,
        sphere: &Sphere,
        vel: &Vec3f,
        triangle: &Triangle,
    ) -> Option<ContactInfo> {
        let normal = self.verts[triangle.normal as usize];
        let triangle = [
            self.verts[triangle.v1 as usize],
            self.verts[triangle.v2 as usize],
            self.verts[triangle.v3 as usize],
        ];
        sphere.sweep_triangle(&triangle, &normal, vel)
    }

    fn link_child(&mut self, child: Child, branch: Branch) {
        let parent_index = *self.node_stack.borrow().last().expect(
            "called link_child on root node",
        );
        let parent = &mut self.nodes[parent_index];

        match branch {
            Branch::Positive => {
                assert_eq!(parent.positive, 0);
                parent.positive = child.pack();
            }
            Branch::Negative => {
                assert_eq!(parent.negative, 0);
                parent.negative = child.pack();
            }
        }
    }

    fn add_polygon<I: IntoIterator<Item = Vec3f>>(&mut self, verts: I, normal: Vec3f) {
        let vert_start = self.verts.len() as u32;
        self.verts.extend(verts);
        let vert_end = self.verts.len() as u32;
        self.verts.push(normal);
        self.triangles.extend(((vert_start + 2)..vert_end).map(|i| {
            Triangle {
                v1: vert_start,
                v2: i - 1,
                v3: i,
                normal: vert_end,
            }
        }));
    }
}

impl Default for World {
    fn default() -> Self {
        World::new()
    }
}

impl LevelVisitor for World {
    fn visit_bsp_root(&mut self, line: &Line2f) {
        let index = self.nodes.len();
        self.nodes.push(Node::new(*line));
        self.node_stack.borrow_mut().push(index);
    }

    fn visit_bsp_node(&mut self, line: &Line2f, branch: Branch) {
        let index = self.nodes.len();
        self.nodes.push(Node::new(*line));
        self.link_child(Child::Node(index), branch);
        self.node_stack.borrow_mut().push(index);
    }

    fn visit_bsp_leaf(&mut self, branch: Branch) {
        let index = self.chunks.len();
        self.chunks.push(Chunk {
            tri_start: self.triangles.len() as u32,
            tri_end: self.triangles.len() as u32,
        });
        self.link_child(Child::Leaf(index), branch);
    }

    fn visit_bsp_leaf_end(&mut self) {
        let chunk = self.chunks.last_mut().expect("missing chunk on end");
        chunk.tri_end = self.triangles.len() as u32;
    }

    fn visit_bsp_node_end(&mut self) {
        self.node_stack.borrow_mut().pop().expect(
            "too many calls to visit_bsp_node_end",
        );
    }

    fn visit_floor_sky_poly(&mut self, &SkyPoly { vertices, height }: &SkyPoly) {
        self.add_polygon(
            vertices.iter().map(|v| Vec3f::new(v[0], height, v[1])),
            Vec3f::new(0.0, 1.0, 0.0),
        );
    }

    fn visit_ceil_sky_poly(&mut self, &SkyPoly { vertices, height }: &SkyPoly) {
        self.add_polygon(
            vertices.iter().rev().map(
                |v| Vec3f::new(v[0], height, v[1]),
            ),
            Vec3f::new(0.0, -1.0, 0.0),
        );
    }

    fn visit_floor_poly(&mut self, &StaticPoly { vertices, height, .. }: &StaticPoly) {
        self.visit_floor_sky_poly(&SkyPoly {
            vertices: vertices,
            height: height,
        });
    }

    fn visit_ceil_poly(&mut self, &StaticPoly { vertices, height, .. }: &StaticPoly) {
        self.visit_ceil_sky_poly(&SkyPoly {
            vertices: vertices,
            height: height,
        });
    }

    fn visit_wall_quad(&mut self,
&StaticQuad { vertices, height_range, blocker, .. }: &StaticQuad){
        if blocker {
            self.visit_sky_quad(&SkyQuad {
                vertices: vertices,
                height_range: height_range,
            });
        }
    }

    fn visit_sky_quad(&mut self, quad: &SkyQuad) {
        let &SkyQuad {
            vertices: &(ref v1, ref v2),
            height_range: (low, high),
        } = quad;
        let edge = (*v2 - *v1).normalized();
        let normal = Vec3f::new(-edge[1], 0.0, edge[0]);
        self.add_polygon(
            Some(Vec3f::new(v1[0], low, v1[1]))
                .into_iter()
                .chain(Some(Vec3f::new(v2[0], low, v2[1])))
                .chain(Some(Vec3f::new(v2[0], high, v2[1])))
                .chain(Some(Vec3f::new(v1[0], high, v1[1]))),
            normal,
        );
    }
}


struct Chunk {
    tri_start: u32,
    tri_end: u32,
}

struct Triangle {
    v1: u32,
    v2: u32,
    v3: u32,
    normal: u32,
}

struct Node {
    partition: Line2f,
    positive: i32,
    negative: i32,
}


#[derive(Eq, PartialEq, Copy, Clone, Debug)]
enum Child {
    Leaf(usize),
    Node(usize),
    Stump,
}

impl Child {
    fn pack(self) -> i32 {
        match self {
            Child::Leaf(index) => {
                assert!(index < i32::MAX as usize);
                -(index as i32)
            }
            Child::Node(index) => {
                assert!(index < i32::MAX as usize);
                (index as i32)
            }
            Child::Stump => 0,
        }
    }

    fn unpack(packed: i32) -> Self {
        if packed == 0 {
            Child::Stump
        } else if packed > 0 {
            Child::Node(packed as usize)
        } else {
            Child::Leaf((-packed) as usize)
        }
    }
}

impl Node {
    fn new(partition: Line2f) -> Node {
        Node {
            partition: partition,
            positive: 0,
            negative: 0,
        }
    }

    fn intersect_sphere(&self, sphere: &Sphere, vel: &Vec3f) -> NodeIntersectIter {
        let Sphere { ref center, radius } = *sphere;
        let dist1 = self.partition.signed_distance(
            &Vec2f::new(center[0], center[2]),
        );
        let dist2 = self.partition.signed_distance(&Vec2f::new(
            center[0] + vel[0],
            center[2] + vel[2],
        ));

        let pos = if dist1 >= -radius || dist2 >= -radius {
            Some(Child::unpack(self.positive))
        } else {
            None
        };

        let neg = if dist1 <= radius || dist2 <= radius {
            Some(Child::unpack(self.negative))
        } else {
            None
        };

        pos.into_iter().chain(neg)
    }
}

type NodeIntersectIter = ::std::iter::Chain<
    ::std::option::IntoIter<Child>,
    ::std::option::IntoIter<Child>,
>;
