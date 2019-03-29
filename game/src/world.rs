use engine::{Entity, EntityId, Transforms};
use idcontain::IdMapVec;
use math::prelude::*;
use math::{ContactInfo, Line2f, Pnt2f, Pnt3f, Sphere, Trans3, Vec3f};
use std::cell::RefCell;
use std::{f32, i32};
use vec_map::VecMap;
use wad::{Branch, LevelVisitor, ObjectId, SkyPoly, SkyQuad, StaticPoly, StaticQuad};

pub struct World {
    nodes: Vec<Node>,
    chunks: Vec<Chunk>,
    triangles: Vec<Triangle>,
    verts: Vec<Pnt3f>,

    dynamic_chunks: IdMapVec<Entity, DynamicChunk>,

    node_stack: RefCell<Vec<usize>>,
}

impl World {
    pub fn update(&mut self, transforms: &Transforms) {
        for index in 0..self.dynamic_chunks.len() {
            let id = self
                .dynamic_chunks
                .index_to_id(index)
                .expect("bad index in iteration");
            let dynamic = self
                .dynamic_chunks
                .get_mut_by_index(index)
                .expect("bad index in iteration");
            dynamic.inverse_transform = transforms
                .get_absolute(id)
                .expect("dynamic chunk missing transform")
                .inverse_transform()
                .expect("singular transform");
        }
    }

    pub fn sweep_sphere(&self, sphere: Sphere, vel: Vec3f) -> Option<ContactInfo> {
        let mut first_contact = ContactInfo {
            time: f32::INFINITY,
            normal: Vec3f::zero(),
        };

        // Statics.
        let mut stack = self.node_stack.borrow_mut();
        stack.push(0);
        while let Some(index) = stack.pop() {
            for child in self.nodes[index].intersect_sphere(sphere, vel) {
                let chunk = match child {
                    Child::Node(index) => {
                        stack.push(index);
                        continue;
                    }
                    Child::Leaf(index) => self.chunks[index],
                };
                self.sweep_chunk(&mut first_contact, chunk, sphere, vel);
            }
        }

        // Dynamics.
        for dynamic in self.dynamic_chunks.access() {
            let transformed_sphere = Sphere {
                center: dynamic.inverse_transform.transform_point(sphere.center),
                radius: sphere.radius,
            };
            let transformed_velocity = dynamic.inverse_transform.transform_vector(vel);
            self.sweep_chunk(
                &mut first_contact,
                dynamic.chunk,
                transformed_sphere,
                transformed_velocity,
            );
        }

        if first_contact.time < f32::INFINITY {
            Some(first_contact)
        } else {
            None
        }
    }

    fn sweep_chunk(
        &self,
        first_contact: &mut ContactInfo,
        chunk: Chunk,
        sphere: Sphere,
        vel: Vec3f,
    ) {
        let tris = &self.triangles[chunk.tri_start as usize..chunk.tri_end as usize];
        *first_contact = tris
            .iter()
            .filter_map(|&tri| self.sweep_sphere_triangle(sphere, vel, tri))
            .fold(*first_contact, |first, current| {
                if first.time < current.time {
                    first
                } else {
                    current
                }
            });
    }

    fn sweep_sphere_triangle(
        &self,
        sphere: Sphere,
        vel: Vec3f,
        triangle: Triangle,
    ) -> Option<ContactInfo> {
        let normal = self.verts[triangle.normal as usize].to_vec();
        let triangle = [
            self.verts[triangle.v1 as usize],
            self.verts[triangle.v2 as usize],
            self.verts[triangle.v3 as usize],
        ];
        sphere.sweep_triangle(&triangle, normal, vel)
    }
}

#[derive(Copy, Clone)]
struct Chunk {
    tri_start: u32,
    tri_end: u32,
}

struct DynamicChunk {
    chunk: Chunk,
    inverse_transform: Trans3,
}

#[derive(Copy, Clone)]
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
        }
    }

    fn unpack(packed: i32) -> Self {
        if packed > 0 {
            Child::Node(packed as usize)
        } else {
            Child::Leaf((-packed) as usize)
        }
    }
}

impl Node {
    fn new(partition: Line2f) -> Node {
        Node {
            partition,
            positive: 0,
            negative: 0,
        }
    }

    fn intersect_sphere(&self, sphere: Sphere, vel: Vec3f) -> NodeIntersectIter {
        let Sphere { center, radius } = sphere;
        let dist1 = self
            .partition
            .signed_distance(Pnt2f::new(center.x, center.z));
        let dist2 = self
            .partition
            .signed_distance(Pnt2f::new(center.x + vel.x, center.z + vel.z));

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

type NodeIntersectIter =
    ::std::iter::Chain<::std::option::IntoIter<Child>, ::std::option::IntoIter<Child>>;

pub struct WorldBuilder<'a> {
    nodes: Vec<Node>,
    chunks: Vec<Chunk>,
    verts: Vec<Pnt3f>,
    node_stack: RefCell<Vec<usize>>,

    triangles: VecMap<Vec<Triangle>>,
    objects: &'a [EntityId],
}

impl<'a> WorldBuilder<'a> {
    pub fn new(objects: &'a [EntityId]) -> Self {
        let mut triangles = VecMap::with_capacity(objects.len() - 1);
        triangles.insert(0, Vec::with_capacity(16_384));
        Self {
            nodes: Vec::with_capacity(128),
            chunks: Vec::with_capacity(128),
            verts: Vec::with_capacity(4096),
            node_stack: RefCell::new(Vec::with_capacity(32)),
            triangles,
            objects,
        }
    }

    pub fn build(self) -> World {
        let mut dynamic_chunks = IdMapVec::with_capacity(self.objects.len() - 1);
        let mut triangles = Vec::with_capacity(self.triangles.values().map(Vec::len).sum());
        for (i_object, object_triangles) in self.triangles {
            let tri_start = triangles.len() as u32;
            triangles.extend(object_triangles);
            if i_object > 0 {
                let tri_end = triangles.len() as u32;
                dynamic_chunks.insert(
                    self.objects[i_object],
                    DynamicChunk {
                        chunk: Chunk { tri_start, tri_end },
                        inverse_transform: Trans3::one(),
                    },
                );
            }
        }

        World {
            nodes: self.nodes,
            chunks: self.chunks,
            verts: self.verts,
            node_stack: self.node_stack,
            dynamic_chunks,
            triangles,
        }
    }

    fn link_child(&mut self, child: Child, branch: Branch) {
        let parent_index = *self
            .node_stack
            .borrow()
            .last()
            .expect("called link_child on root node");
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

    fn add_polygon<I: IntoIterator<Item = Pnt3f>>(
        &mut self,
        object_id: ObjectId,
        verts: I,
        normal: Pnt3f,
    ) {
        let triangles = &mut self
            .triangles
            .entry(object_id.0 as usize)
            .or_insert_with(|| Vec::with_capacity(256));
        let vert_start = self.verts.len() as u32;
        self.verts.extend(verts);
        let vert_end = self.verts.len() as u32;
        self.verts.push(normal);
        triangles.extend(((vert_start + 2)..vert_end).map(|i| Triangle {
            v1: vert_start,
            v2: i - 1,
            v3: i,
            normal: vert_end,
        }));
    }
}

impl<'a> LevelVisitor for WorldBuilder<'a> {
    fn visit_bsp_root(&mut self, line: &Line2f) {
        assert_eq!(self.nodes.len(), 0);
        self.nodes.push(Node::new(*line));
        self.node_stack.borrow_mut().push(0);
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
            tri_start: self.triangles[0].len() as u32,
            tri_end: self.triangles[0].len() as u32,
        });
        self.link_child(Child::Leaf(index), branch);
    }

    fn visit_bsp_leaf_end(&mut self) {
        let chunk = self.chunks.last_mut().expect("missing chunk on end");
        chunk.tri_end = self.triangles[0].len() as u32;
    }

    fn visit_bsp_node_end(&mut self) {
        self.node_stack
            .borrow_mut()
            .pop()
            .expect("too many calls to visit_bsp_node_end");
    }

    fn visit_floor_sky_poly(&mut self, poly: &SkyPoly) {
        self.add_polygon(
            poly.object_id,
            poly.vertices
                .iter()
                .map(|v| Pnt3f::new(v[0], poly.height, v[1])),
            Pnt3f::new(0.0, 1.0, 0.0),
        );
    }

    fn visit_ceil_sky_poly(&mut self, poly: &SkyPoly) {
        self.add_polygon(
            poly.object_id,
            poly.vertices
                .iter()
                .rev()
                .map(|v| Pnt3f::new(v[0], poly.height, v[1])),
            Pnt3f::new(0.0, -1.0, 0.0),
        );
    }

    fn visit_floor_poly(&mut self, poly: &StaticPoly) {
        self.visit_floor_sky_poly(&SkyPoly {
            vertices: poly.vertices,
            height: poly.height,
            object_id: poly.object_id,
        });
    }

    fn visit_ceil_poly(&mut self, poly: &StaticPoly) {
        self.visit_ceil_sky_poly(&SkyPoly {
            object_id: poly.object_id,
            vertices: poly.vertices,
            height: poly.height,
        });
    }

    fn visit_wall_quad(&mut self, quad: &StaticQuad) {
        if quad.blocker {
            self.visit_sky_quad(&SkyQuad {
                object_id: quad.object_id,
                vertices: quad.vertices,
                height_range: quad.height_range,
            });
        }
    }

    fn visit_sky_quad(&mut self, quad: &SkyQuad) {
        let &SkyQuad {
            object_id,
            vertices: (v1, v2),
            height_range: (low, high),
        } = quad;
        let edge = (v2 - v1).normalize_or_zero();
        let normal = Pnt3f::new(-edge[1], 0.0, edge[0]);
        self.add_polygon(
            object_id,
            [
                Pnt3f::new(v1[0], low, v1[1]),
                Pnt3f::new(v2[0], low, v2[1]),
                Pnt3f::new(v2[0], high, v2[1]),
                Pnt3f::new(v1[0], high, v1[1]),
            ]
            .iter()
            .cloned(),
            normal,
        );
    }
}
