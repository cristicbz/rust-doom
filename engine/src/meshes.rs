use super::entities::{Entities, Entity, EntityId};
use super::errors::{ErrorKind, Result};
use super::system::InfallibleSystem;
use super::window::Window;
pub use glium::index::IndexBuffer;
use glium::index::{IndicesSource, PrimitiveType};
use glium::vertex::{Vertex, VertexBuffer, VerticesSource};
pub use glium_typed_buffer_any::TypedVertexBufferAny;
use idcontain::IdMapVec;
use log::{debug, error};

#[derive(Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Copy, Clone)]
pub struct MeshId(EntityId);

pub struct Meshes {
    map: IdMapVec<Entity, Mesh>,
}

impl Meshes {
    pub fn add<'a>(
        &'a mut self,
        window: &'a Window,
        entities: &'a mut Entities,
        parent: EntityId,
        name: &'static str,
    ) -> MeshAdder<'a, (), ()> {
        MeshAdder {
            context: MeshAdderContext {
                meshes: self,
                window,
                entities,
                parent,
                name,
            },
            vertices: (),
            indices: (),
        }
    }

    pub fn get(&self, mesh_id: MeshId) -> Option<MeshRef> {
        self.map.get(mesh_id.0).map(|mesh| match mesh.data {
            InternalMeshData::Owned {
                ref vertices,
                ref indices,
            } => MeshRef {
                vertices,
                indices: indices.as_ref(),
            },
            InternalMeshData::Inherit {
                vertices_from,
                ref indices,
            } => MeshRef {
                vertices: match self
                    .map
                    .get(vertices_from.0)
                    .expect("missing mesh in stored vertices_from")
                    .data
                {
                    InternalMeshData::Owned { ref vertices, .. } => vertices,
                    _ => panic!("unowned mesh in stored vertices_from"),
                },
                indices: Some(indices),
            },
        })
    }

    pub fn get_mut(&mut self, mesh_id: MeshId) -> Option<MeshRefMut> {
        self.map.get_mut(mesh_id.0).map(|mesh| match mesh.data {
            InternalMeshData::Owned {
                ref mut vertices,
                ref mut indices,
            } => MeshRefMut {
                vertices: Some(vertices),
                indices: indices.as_mut(),
            },
            InternalMeshData::Inherit {
                vertices_from: _vertices_from,
                ref mut indices,
            } => MeshRefMut {
                vertices: None,
                indices: Some(indices),
            },
        })
    }
}

pub struct MeshRefMut<'a> {
    pub vertices: Option<&'a mut TypedVertexBufferAny>,
    pub indices: Option<&'a mut IndexBuffer<u32>>,
}

pub struct MeshRef<'a> {
    vertices: &'a TypedVertexBufferAny,
    indices: Option<&'a IndexBuffer<u32>>,
}

impl<'a, 'b: 'a> Into<IndicesSource<'a>> for &'a MeshRef<'b> {
    fn into(self) -> IndicesSource<'a> {
        self.indices.map_or(
            IndicesSource::NoIndices {
                primitives: PrimitiveType::TrianglesList,
            },
            |indices| indices.into(),
        )
    }
}

impl<'a, 'b: 'a> Into<VerticesSource<'a>> for &'a MeshRef<'b> {
    fn into(self) -> VerticesSource<'a> {
        self.vertices.into()
    }
}

#[must_use]
pub struct MeshAdder<'a, VertexDataT, IndexDataT> {
    context: MeshAdderContext<'a>,
    vertices: VertexDataT,
    indices: IndexDataT,
}

pub struct OwnedVertexData(TypedVertexBufferAny);
pub struct SharedVertexData(MeshId);
pub struct IndexData(IndexBuffer<u32>);

impl<'a, IndexDataT> MeshAdder<'a, (), IndexDataT> {
    pub fn immutable<VertexT>(
        self,
        vertices: &[VertexT],
    ) -> Result<MeshAdder<'a, OwnedVertexData, IndexDataT>>
    where
        VertexT: Vertex + Send + 'static,
    {
        Ok(MeshAdder {
            vertices: OwnedVertexData(
                VertexBuffer::immutable(self.context.window.facade(), vertices)
                    .map_err(ErrorKind::glium(self.context.name))?
                    .into(),
            ),
            indices: self.indices,
            context: self.context,
        })
    }

    pub fn persistent<VertexT>(
        self,
        vertices: &[VertexT],
    ) -> Result<MeshAdder<'a, OwnedVertexData, IndexDataT>>
    where
        VertexT: Vertex + Send + 'static,
    {
        Ok(MeshAdder {
            vertices: OwnedVertexData(
                VertexBuffer::persistent(self.context.window.facade(), vertices)
                    .map_err(ErrorKind::glium(self.context.name))?
                    .into(),
            ),
            indices: self.indices,
            context: self.context,
        })
    }

    pub fn shared(self, vertices_from: MeshId) -> MeshAdder<'a, SharedVertexData, IndexDataT> {
        // TODO(cristicbz): If parent comes from a different `Meshes` object, the assertion that
        // the parent is in the map is in incorrect.
        let mut owner = vertices_from;
        while let InternalMeshData::Inherit { vertices_from, .. } = self
            .context
            .meshes
            .map
            .get(owner.0)
            .expect("missing mesh for MeshId")
            .data
        {
            owner = vertices_from;
        }
        MeshAdder {
            vertices: SharedVertexData(owner),
            indices: self.indices,
            context: self.context,
        }
    }
}

impl<'a, VertexDataT> MeshAdder<'a, VertexDataT, ()> {
    pub fn immutable_indices(
        self,
        indices: &[u32],
    ) -> Result<MeshAdder<'a, VertexDataT, IndexData>> {
        Ok(MeshAdder {
            indices: IndexData(
                IndexBuffer::persistent(
                    self.context.window.facade(),
                    PrimitiveType::TrianglesList,
                    indices,
                )
                .map_err(ErrorKind::glium(self.context.name))?,
            ),
            vertices: self.vertices,
            context: self.context,
        })
    }

    pub fn persistent_indices(
        self,
        indices: &[u32],
    ) -> Result<MeshAdder<'a, VertexDataT, IndexData>> {
        Ok(MeshAdder {
            indices: IndexData(
                IndexBuffer::persistent(
                    self.context.window.facade(),
                    PrimitiveType::TrianglesList,
                    indices,
                )
                .map_err(ErrorKind::glium(self.context.name))?,
            ),
            vertices: self.vertices,
            context: self.context,
        })
    }
}

impl<'a> MeshAdder<'a, OwnedVertexData, ()> {
    pub fn build_unindexed(self) -> Result<MeshId> {
        self.context.add(InternalMeshData::Owned {
            vertices: self.vertices.0,
            indices: None,
        })
    }
}

impl<'a> MeshAdder<'a, SharedVertexData, IndexData> {
    pub fn build(self) -> Result<MeshId> {
        self.context.add(InternalMeshData::Inherit {
            vertices_from: self.vertices.0,
            indices: self.indices.0,
        })
    }
}

impl<'a> MeshAdder<'a, OwnedVertexData, IndexData> {
    pub fn build(self) -> Result<MeshId> {
        self.context.add(InternalMeshData::Owned {
            vertices: self.vertices.0,
            indices: Some(self.indices.0),
        })
    }
}

pub struct Mesh {
    data: InternalMeshData,
}

impl<'context> InfallibleSystem<'context> for Meshes {
    type Dependencies = &'context Entities;

    fn debug_name() -> &'static str {
        "meshes"
    }

    fn create(_deps: &Entities) -> Self {
        Meshes {
            map: IdMapVec::with_capacity(128),
        }
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
        if !self.map.is_empty() {
            error!("Meshes leaked, {} instances.", self.map.len());
        }
    }
}

#[cfg_attr(feature = "cargo-clippy", allow(clippy::large_enum_variant))]
enum InternalMeshData {
    Owned {
        vertices: TypedVertexBufferAny,
        indices: Option<IndexBuffer<u32>>,
    },
    Inherit {
        vertices_from: MeshId,
        indices: IndexBuffer<u32>,
    },
}

struct MeshAdderContext<'a> {
    meshes: &'a mut Meshes,
    window: &'a Window,
    entities: &'a mut Entities,
    parent: EntityId,
    name: &'static str,
}

impl<'a> MeshAdderContext<'a> {
    fn add(self, data: InternalMeshData) -> Result<MeshId> {
        let id = self.entities.add(self.parent, self.name)?;
        self.meshes.map.insert(id, Mesh { data });
        debug!(
            "Added mesh {:?} {:?} as child of {:?}.",
            self.name, id, self.parent
        );
        Ok(MeshId(id))
    }
}
