use super::entities::{Entities, Entity, EntityId};
use super::system::InfallibleSystem;
use idcontain::IdMapVec;
use log::{debug, error};
use math::{self, Mat4, Rad};

#[derive(Copy, Clone, Debug)]
pub struct Projection {
    pub fov: Rad<f32>,
    pub aspect_ratio: f32,
    pub near: f32,
    pub far: f32,
}

pub struct Projections {
    map: IdMapVec<Entity, StoredProjection>,
}

impl Projections {
    pub fn attach(&mut self, entity: EntityId, projection: Projection) {
        let old = self.map.insert(
            entity,
            StoredProjection {
                projection,
                matrix: projection.into(),
            },
        );
        if old.is_some() {
            error!(
                "Entity {:?} already had a projection attached, replacing.",
                entity
            );
        }
    }

    pub fn get_matrix(&self, entity: EntityId) -> Option<&Mat4> {
        self.map.get(entity).map(|stored| &stored.matrix)
    }

    pub fn replace_with<F, O>(&mut self, entity: EntityId, with: F) -> O
    where
        F: FnOnce(Option<&mut Projection>) -> O,
    {
        let stored = self.map.get_mut(entity);
        if let Some(stored) = stored {
            let output = with(Some(&mut stored.projection));
            stored.matrix = stored.projection.into();
            output
        } else {
            with(None)
        }
    }
}

impl<'context> InfallibleSystem<'context> for Projections {
    type Dependencies = &'context Entities;

    fn debug_name() -> &'static str {
        "projections"
    }

    fn create(_: &'context Entities) -> Self {
        Projections {
            map: IdMapVec::with_capacity(128),
        }
    }

    fn update(&mut self, entities: &Entities) {
        for &entity in entities.last_removed() {
            if self.map.remove(entity).is_some() {
                debug!("Removed projection {:?}.", entity);
            }
        }
    }

    fn teardown(&mut self, entities: &Entities) {
        self.update(entities);
    }

    fn destroy(mut self, entities: &Entities) {
        self.update(entities);
        if !self.map.is_empty() {
            error!("Projections leaked, {} instances.", self.map.len());
        }
    }
}

struct StoredProjection {
    projection: Projection,
    matrix: Mat4,
}

impl Into<Mat4> for Projection {
    fn into(self) -> Mat4 {
        math::perspective(self.fov, self.aspect_ratio, self.near, self.far)
    }
}
