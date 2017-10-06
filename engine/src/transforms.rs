use super::entities::{Entities, EntityId, Entity};
use super::errors::{Error, ErrorKind};
use super::system::InfallibleSystem;
use idcontain::IdMap;
use math::{Transform as MathTransform, Mat4};

derive_flat! {
    #[element(Transform, &TransformRef, &mut TransformMut)]
    #[access(&TransformsRef, &mut TransformsMut)]
    pub struct TransformsAccess {
        #[element(local)]
        pub locals: Vec<MathTransform>,

        #[element(absolute)]
        pub absolutes: Vec<MathTransform>,

        #[element(absolute_matrix)]
        pub absolute_matrices: Vec<Mat4>,
    }
}

pub struct Transforms {
    map: IdMap<Entity, TransformsAccess>,
    removed: Vec<usize>,
}

impl Transforms {
    pub fn attach_identity(&mut self, entity: EntityId) {
        self.attach(entity, MathTransform::default())
    }

    pub fn attach(&mut self, entity: EntityId, transform: MathTransform) {
        let old = self.map.insert(
            entity,
            Transform {
                local: transform,
                absolute: MathTransform::default(),
                absolute_matrix: Mat4::new_identity(),
            },
        );
        if old.is_some() {
            error!(
                "Entity {:?} already had a transform attached, replacing.",
                entity
            );
        }
    }

    pub fn get_local_mut(&mut self, entity: EntityId) -> Option<&mut MathTransform> {
        self.map.get_mut(entity).map(|transform| transform.local)
    }

    pub fn get_absolute(&self, entity: EntityId) -> Option<&MathTransform> {
        self.map.get(entity).map(|transform| transform.absolute)
    }

    pub fn get_absolute_matrix(&self, entity: EntityId) -> Option<&Mat4> {
        self.map.get(entity).map(
            |transform| transform.absolute_matrix,
        )
    }

    fn lookup_parent(&self, entities: &Entities, id: EntityId) -> ParentLookup {
        let mut id = id;
        loop {
            match entities.parent_of(id) {
                Ok(Some(parent_id)) => {
                    if let Some(parent_index) = self.map.id_to_index(parent_id) {
                        return ParentLookup::Found {
                            parent_id,
                            parent_index,
                        };
                    } else {
                        id = parent_id;
                    }
                }
                Ok(None) => return ParentLookup::IsRoot,
                Err(Error(ErrorKind::NoSuchEntity(..), _)) => {
                    return ParentLookup::Removed;
                }
                Err(error) => panic!("unexpected error in `parent_of`: {}", error),
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
            let mut id = self.map.index_to_id(index).expect(
                "misleading map length: index_to_id",
            );
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
                                parent_id,
                                parent_index,
                                id,
                                index
                            );
                            self.map.swap_indices(parent_index, index);
                            id = parent_id;
                            continue;
                        }
                        let access = self.map.access_mut();
                        access.absolutes[index] =
                            access.absolutes[parent_index].then(&access.locals[index]);
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
            self.map.remove_by_index(index);
        }

        let access = self.map.access_mut();
        for (matrix, transform) in access.absolute_matrices.iter_mut().zip(&*access.absolutes) {
            *matrix = Mat4::from(transform);
        }
    }

    fn teardown(&mut self, entities: &Entities) {
        self.update(entities);
    }

    fn destroy(mut self, entities: &Entities) {
        self.update(entities);
        if self.map.len() > 0 {
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
