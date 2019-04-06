use super::entities::{Entities, Entity, EntityId};
use super::system::InfallibleSystem;
use idcontain::{derive_flat, IdMap};
use log::{debug, error};
use math::prelude::*;
use math::Trans3;

derive_flat! {
    #[element(Transform, &TransformRef, &mut TransformMut)]
    #[access(&TransformsRef, &mut TransformsMut)]
    pub struct TransformsAccess {
        #[element(local)]
        pub locals: Vec<Trans3>,

        #[element(absolute)]
        pub absolutes: Vec<Trans3>,
    }
}

pub struct Transforms {
    map: IdMap<Entity, TransformsAccess>,
    removed: Vec<usize>,
}

impl Transforms {
    pub fn attach_identity(&mut self, entity: EntityId) {
        self.attach(entity, Trans3::one())
    }

    pub fn attach(&mut self, entity: EntityId, transform: Trans3) {
        let old = self.map.insert(
            entity,
            Transform {
                local: transform,
                absolute: Trans3::one(),
            },
        );
        if old.is_some() {
            error!(
                "Entity {:?} already had a transform attached, replacing.",
                entity
            );
        }
    }

    pub fn get_local_mut(&mut self, entity: EntityId) -> Option<&mut Trans3> {
        self.map.get_mut(entity).map(|transform| transform.local)
    }

    pub fn get_absolute(&self, entity: EntityId) -> Option<&Trans3> {
        self.map.get(entity).map(|transform| transform.absolute)
    }

    fn lookup_parent(&self, entities: &Entities, id: EntityId) -> ParentLookup {
        let mut id = id;
        loop {
            let parent_id = match entities.get(id) {
                None => return ParentLookup::Removed,
                Some(entity) => match entity.parent() {
                    None => return ParentLookup::IsRoot,
                    Some(parent_id) => parent_id,
                },
            };
            match self.map.id_to_index(parent_id) {
                Some(parent_index) => {
                    return ParentLookup::Found {
                        parent_id,
                        parent_index,
                    };
                }
                None => id = parent_id,
            }
        }
    }
}

impl<'context> InfallibleSystem<'context> for Transforms {
    type Dependencies = &'context Entities;

    fn debug_name() -> &'static str {
        "transforms"
    }

    fn create(_deps: &Entities) -> Self {
        Transforms {
            map: IdMap::with_capacity(1024),
            removed: Vec::with_capacity(128),
        }
    }

    fn update(&mut self, entities: &Entities) {
        for index in 0..self.map.len() {
            let mut id = self
                .map
                .index_to_id(index)
                .expect("misleading map length: index_to_id");
            loop {
                match self.lookup_parent(entities, id) {
                    ParentLookup::IsRoot => {
                        let access = self.map.access_mut();
                        access.absolutes[index] = access.locals[index];
                        break;
                    }
                    ParentLookup::Found {
                        parent_id,
                        parent_index,
                    } => {
                        assert_ne!(parent_index, index);
                        if parent_index > index {
                            debug!(
                                "Parent {:?} @ {} and child {:?} @ {} have reversed transforms, \
                                 swapping.",
                                parent_id, parent_index, id, index
                            );
                            self.map.swap_indices(parent_index, index);
                            id = parent_id;
                            continue;
                        }
                        let access = self.map.access_mut();
                        access.absolutes[index] =
                            access.absolutes[parent_index].concat(&access.locals[index]);
                        break;
                    }
                    ParentLookup::Removed => {
                        debug!("Transform {:?} @ {} lazily removed.", id, index);
                        self.removed.push(index);
                        break;
                    }
                }
            }
        }

        for &index in self.removed.iter().rev() {
            debug!(
                "Actually removed transform for {:?} @ {}.",
                self.map.index_to_id(index).unwrap(),
                index
            );
            self.map.remove_by_index(index);
        }
        self.removed.clear();
    }

    fn teardown(&mut self, entities: &Entities) {
        self.update(entities);
    }

    fn destroy(mut self, entities: &Entities) {
        self.update(entities);
        if !self.map.is_empty() {
            error!("Transforms leaked, {} instances.", self.map.len());
        }
    }
}

enum ParentLookup {
    Removed,
    IsRoot,
    Found {
        parent_id: EntityId,
        parent_index: usize,
    },
}
