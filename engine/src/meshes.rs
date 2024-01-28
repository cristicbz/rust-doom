use crate::Shaders;

use super::entities::{Entities, Entity, EntityId};
use super::errors::Result;
use super::system::InfallibleSystem;
use bytemuck::Pod;
use cgmath::SquareMatrix;
use idcontain::IdMapVec;
use log::{debug, error};
use math::Mat4;
use wgpu::util::DeviceExt;

#[derive(Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Copy, Clone)]
pub struct MeshId(EntityId);

pub struct Meshes {
    map: IdMapVec<Entity, Mesh>,
}

impl Meshes {
    pub fn add<'a>(
        &'a mut self,
        entities: &'a mut Entities,
        parent: EntityId,
        name: &'static str,
        device: &wgpu::Device,
        shaders: &Shaders,
    ) -> MeshAdder<'a, (), ()> {
        let model_transform: [[f32; 4]; 4] = Mat4::identity().into();
        let model_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&model_transform),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let right_vector = [1.0f32, 0.0, 0.0];
        let right_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&right_vector),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let bind_group = create_bind_group(device, shaders, &model_buffer, &right_buffer);
        MeshAdder {
            context: MeshAdderContext {
                meshes: self,
                entities,
                parent,
                name,
            },
            vertices: (),
            indices: (),
            model_buffer,
            right_buffer,
            bind_group,
        }
    }

    pub fn get(&self, mesh_id: MeshId) -> Option<MeshRef> {
        self.map.get(mesh_id.0).map(|mesh| match mesh.data {
            InternalMeshData::Owned {
                ref vertices,
                ref indices,
                model_buffer: _,
                right_buffer: _,
                ref bind_group,
            } => MeshRef {
                vertices,
                indices: indices.as_ref(),
                bind_group,
            },
            InternalMeshData::Inherit {
                vertices_from,
                ref indices,
                model_buffer: _,
                right_buffer: _,
                ref bind_group,
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
                bind_group,
            },
        })
    }

    pub fn get_mut(&mut self, mesh_id: MeshId) -> Option<MeshRefMut> {
        self.map.get_mut(mesh_id.0).map(|mesh| match mesh.data {
            InternalMeshData::Owned {
                ref mut vertices,
                ref mut indices,
                model_buffer: _,
                right_buffer: _,
                bind_group: _,
            } => MeshRefMut {
                vertices: Some(vertices),
                indices: indices.as_mut(),
            },
            InternalMeshData::Inherit {
                vertices_from: _vertices_from,
                ref mut indices,
                model_buffer: _,
                right_buffer: _,
                bind_group: _,
            } => MeshRefMut {
                vertices: None,
                indices: Some(indices),
            },
        })
    }
}

pub struct MeshRefMut<'a> {
    pub vertices: Option<&'a mut wgpu::Buffer>,
    pub indices: Option<&'a mut wgpu::Buffer>,
}

pub struct MeshRef<'a> {
    vertices: &'a wgpu::Buffer,
    indices: Option<&'a wgpu::Buffer>,
    bind_group: &'a wgpu::BindGroup,
}

impl<'a> MeshRef<'a> {
    pub(crate) fn vertex_buffer(&self) -> wgpu::BufferSlice<'a> {
        self.vertices.slice(..)
    }

    pub(crate) fn index_buffer(&self) -> wgpu::BufferSlice<'a> {
        self.indices.expect("index buffer not present").slice(..)
    }

    pub(crate) fn index_count(&self) -> u32 {
        self.indices.expect("index buffer not present").size() as u32 / 4
    }

    pub(crate) fn bind_group(&self) -> &'a wgpu::BindGroup {
        self.bind_group
    }
}

#[must_use]
pub struct MeshAdder<'a, VertexDataT, IndexDataT> {
    context: MeshAdderContext<'a>,
    vertices: VertexDataT,
    indices: IndexDataT,
    model_buffer: wgpu::Buffer,
    right_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

pub struct OwnedVertexData(wgpu::Buffer);
pub struct SharedVertexData(MeshId);
pub struct IndexData(wgpu::Buffer);

impl<'a, IndexDataT> MeshAdder<'a, (), IndexDataT> {
    pub fn immutable<VertexT>(
        self,
        vertices: &[VertexT],
        device: &wgpu::Device,
    ) -> Result<MeshAdder<'a, OwnedVertexData, IndexDataT>>
    where
        VertexT: Pod + Send + 'static,
    {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        Ok(MeshAdder {
            vertices: OwnedVertexData(vertex_buffer),
            context: self.context,
            indices: self.indices,
            model_buffer: self.model_buffer,
            right_buffer: self.right_buffer,
            bind_group: self.bind_group,
        })
    }

    pub fn persistent<VertexT>(
        self,
        vertices: &[VertexT],
        device: &wgpu::Device,
    ) -> Result<MeshAdder<'a, OwnedVertexData, IndexDataT>>
    where
        VertexT: Pod + Send + 'static,
    {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        Ok(MeshAdder {
            vertices: OwnedVertexData(vertex_buffer),
            context: self.context,
            indices: self.indices,
            model_buffer: self.model_buffer,
            right_buffer: self.right_buffer,
            bind_group: self.bind_group,
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
            context: self.context,
            indices: self.indices,
            model_buffer: self.model_buffer,
            right_buffer: self.right_buffer,
            bind_group: self.bind_group,
        }
    }
}

impl<'a, VertexDataT> MeshAdder<'a, VertexDataT, ()> {
    pub fn immutable_indices(
        self,
        indices: &[u32],
        device: &wgpu::Device,
    ) -> Result<MeshAdder<'a, VertexDataT, IndexData>> {
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        Ok(MeshAdder {
            indices: IndexData(index_buffer),
            context: self.context,
            vertices: self.vertices,
            model_buffer: self.model_buffer,
            right_buffer: self.right_buffer,
            bind_group: self.bind_group,
        })
    }

    pub fn persistent_indices(
        self,
        indices: &[u32],
        device: &wgpu::Device,
    ) -> Result<MeshAdder<'a, VertexDataT, IndexData>> {
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        Ok(MeshAdder {
            indices: IndexData(index_buffer),
            context: self.context,
            vertices: self.vertices,
            model_buffer: self.model_buffer,
            right_buffer: self.right_buffer,
            bind_group: self.bind_group,
        })
    }
}

impl<'a> MeshAdder<'a, OwnedVertexData, ()> {
    pub fn build_unindexed(self) -> Result<MeshId> {
        self.context.add(InternalMeshData::Owned {
            vertices: self.vertices.0,
            indices: None,
            model_buffer: self.model_buffer,
            right_buffer: self.right_buffer,
            bind_group: self.bind_group,
        })
    }
}

impl<'a> MeshAdder<'a, SharedVertexData, IndexData> {
    pub fn build(self) -> Result<MeshId> {
        self.context.add(InternalMeshData::Inherit {
            vertices_from: self.vertices.0,
            indices: self.indices.0,
            model_buffer: self.model_buffer,
            right_buffer: self.right_buffer,
            bind_group: self.bind_group,
        })
    }
}

impl<'a> MeshAdder<'a, OwnedVertexData, IndexData> {
    pub fn build(self) -> Result<MeshId> {
        self.context.add(InternalMeshData::Owned {
            vertices: self.vertices.0,
            indices: Some(self.indices.0),
            model_buffer: self.model_buffer,
            right_buffer: self.right_buffer,
            bind_group: self.bind_group,
        })
    }
}

fn create_bind_group(
    device: &wgpu::Device,
    shaders: &Shaders,
    model_buffer: &wgpu::Buffer,
    right_buffer: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: shaders.model_bind_group_layout(),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: model_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: right_buffer.as_entire_binding(),
            },
        ],
    })
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
        vertices: wgpu::Buffer,
        indices: Option<wgpu::Buffer>,
        model_buffer: wgpu::Buffer,
        right_buffer: wgpu::Buffer,
        bind_group: wgpu::BindGroup,
    },
    Inherit {
        vertices_from: MeshId,
        indices: wgpu::Buffer,
        model_buffer: wgpu::Buffer,
        right_buffer: wgpu::Buffer,
        bind_group: wgpu::BindGroup,
    },
}

struct MeshAdderContext<'a> {
    meshes: &'a mut Meshes,
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
