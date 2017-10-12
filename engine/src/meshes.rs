use super::entities::{Entities, Entity, EntityId};
use super::errors::{NeededBy, Result};
use super::system::InfallibleSystem;
use super::window::Window;
use glium::index::{Index, IndexBufferAny, IndexBuffer, IndicesSource, PrimitiveType};
use glium::vertex::{Vertex, VertexBufferAny, IntoVerticesSource, VertexBuffer, VerticesSource};
use idcontain::IdMapVec;

#[derive(Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Copy, Clone)]
pub struct MeshId(EntityId);

pub struct Meshes {
    map: IdMapVec<Entity, Mesh>,
}

impl Meshes {
    pub fn add_immutable<V: Vertex + Send + 'static, I: Index>(
        &mut self,
        window: &Window,
        entities: &mut Entities,
        parent: EntityId,
        name: &'static str,
        vertices: &[V],
        indices: Option<&[I]>,
    ) -> Result<MeshId> {
        let id = entities.add(parent, name)?;
        self.map.insert(
            id,
            Mesh {
                data: MeshData::Owned {
                    vertices: VertexBuffer::immutable(window.facade(), vertices)
                        .needed_by(name)?
                        .into(),
                    indices: if let Some(indices) = indices {
                        Some(
                            IndexBuffer::immutable(
                                window.facade(),
                                PrimitiveType::TrianglesList,
                                indices,
                            ).needed_by(name)?
                                .into(),
                        )
                    } else {
                        None
                    },
                },
            },
        );
        debug!("Added mesh {:?} {:?} as child of {:?}.", name, id, parent);
        Ok(MeshId(id))
    }

    pub fn add_immutable_indices<I: Index>(
        &mut self,
        window: &Window,
        entities: &mut Entities,
        parent: MeshId,
        name: &'static str,
        indices: &[I],
    ) -> Result<MeshId> {
        let id = entities.add(parent.0, name)?;
        // TODO(cristicbz): If parent comes from a different `Meshes` object, the assertion that
        // the parent is in the map is in incorrect.
        let mut owner = parent;
        while let MeshData::Inherit { vertices_from, .. } =
            self.map.get(owner.0).expect("missing mesh for MeshId").data
        {
            owner = vertices_from;
        }
        self.map.insert(
            id,
            Mesh {
                data: MeshData::Inherit {
                    vertices_from: owner,
                    indices: IndexBuffer::immutable(
                        window.facade(),
                        PrimitiveType::TrianglesList,
                        indices,
                    ).needed_by(name)?
                        .into(),
                },
            },
        );
        debug!(
            "Added mesh {:?} {:?} as child of {:?}, with actual owner {:?}.",
            name,
            id,
            parent,
            owner
        );
        Ok(MeshId(id))
    }

    pub fn get(&self, mesh_id: MeshId) -> Option<MeshRef> {
        self.map.get(mesh_id.0).map(|mesh| match mesh.data {
            MeshData::Owned {
                ref vertices,
                ref indices,
            } => MeshRef {
                vertices,
                indices: indices.as_ref(),
            },
            MeshData::Inherit {
                vertices_from,
                ref indices,
            } => MeshRef {
                vertices: match self.map
                    .get(vertices_from.0)
                    .expect("missing mesh in stored vertices_from")
                    .data {
                    MeshData::Owned { ref vertices, .. } => vertices,
                    _ => panic!("unowned mesh in stored vertices_from"),
                },
                indices: Some(indices),
            },
        })
    }
}

pub struct MeshRef<'a> {
    vertices: &'a VertexBufferAny,
    indices: Option<&'a IndexBufferAny>,
}

impl<'a, 'b: 'a> Into<IndicesSource<'a>> for &'a MeshRef<'b> {
    fn into(self) -> IndicesSource<'a> {
        self.indices.map_or(
            IndicesSource::NoIndices { primitives: PrimitiveType::TrianglesList },
            |buffer| buffer.into(),
            )
    }
}

impl<'a, 'b: 'a> IntoVerticesSource<'a> for &'a MeshRef<'b> {
    fn into_vertices_source(self) -> VerticesSource<'a> {
        self.vertices.into_vertices_source()
    }
}

pub struct Mesh {
    data: MeshData,
}

impl<'context> InfallibleSystem<'context> for Meshes {
    type Dependencies = &'context Entities;

    fn debug_name() -> &'static str {
        "meshes"
    }

    fn create(_deps: &Entities) -> Self {
        Meshes { map: IdMapVec::with_capacity(128) }
    }

    fn update(&mut self, entities: &Entities) {
        for &entity in entities.last_removed() {
            if self.map.remove(entity).is_some() {
                debug!("Removed mesh {:?}.", entity);
            }
        }
    }

    fn teardown(&mut self, entities: &Entities) {
        self.update(entities);
    }

    fn destroy(mut self, entities: &Entities) {
        self.update(entities);
        if self.map.len() > 0 {
            error!("Meshes leaked, {} instances.", self.map.len());
        }
    }
}

#[cfg_attr(feature = "cargo-clippy", allow(large_enum_variant))]
enum MeshData {
    Owned {
        vertices: VertexBufferAny,
        indices: Option<IndexBufferAny>,
    },
    Inherit {
        vertices_from: MeshId,
        indices: IndexBufferAny,
    },
}
