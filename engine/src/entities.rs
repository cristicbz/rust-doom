use super::errors::{ErrorKind, Result};
use super::system::InfallibleSystem;
use failchain::bail;
use idcontain::{Id, IdSlab, OptionId};
use log::{debug, error};
use std::fmt::Write;
use std::mem;

pub type EntityId = Id<Entity>;

pub struct Entities {
    slab: IdSlab<Entity>,
    first_root: OptionId<Entity>,
    removed: Vec<EntityId>,
    last_removed: Vec<EntityId>,
}

impl Entities {
    #[inline]
    pub fn len(&self) -> usize {
        self.slab.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.slab.len() == 0
    }

    #[inline]
    pub fn contains(&self, id: EntityId) -> bool {
        self.slab.contains(id)
    }

    #[inline]
    pub fn add_root(&mut self, name: &'static str) -> EntityId {
        let Entities {
            ref mut slab,
            ref mut first_root,
            ..
        } = *self;
        debug!("Adding root {:?}...", name);
        let new_id = slab.insert(Entity {
            name,
            parent: OptionId::none(),
            child: OptionId::none(),

            next: *first_root,
            previous: OptionId::none(),

            liveness: Liveness::Alive,
        });
        debug!("New root id {:?} for {:?}.", new_id, name);
        let old_first_root: Option<EntityId> =
            mem::replace(first_root, OptionId::some(new_id)).into();
        if let Some(old_first_root) = old_first_root {
            let old_entity = &mut slab[old_first_root];
            debug!(
                "Patched previous of root {:?} {:?} to {:?}...",
                old_entity.name, old_first_root, new_id
            );
            old_entity.previous = OptionId::some(new_id);
        }
        debug!("Added root {:?} {:?}...", name, new_id);
        new_id
    }

    #[inline]
    pub fn add(&mut self, parent: EntityId, name: &'static str) -> Result<EntityId> {
        debug!("Adding entity {:?} as child of {:?}...", name, parent);
        let Entities {
            ref mut slab,
            ref mut removed,
            ..
        } = *self;
        let new_id = slab.insert(Entity {
            name,
            parent: OptionId::some(parent),
            child: OptionId::none(),

            next: OptionId::none(),
            previous: OptionId::none(),

            liveness: Liveness::Alive,
        });
        let (parent_exists, parent_dead, old_child) = if let Some(parent) = slab.get_mut(parent) {
            (
                true,
                !parent.liveness.is_alive(),
                mem::replace(&mut parent.child, OptionId::some(new_id)),
            )
        } else {
            (false, false, OptionId::none())
        };

        if !parent_exists {
            slab.remove(new_id);
            bail!(ErrorKind::NoSuchEntity {
                context: "add",
                needed_by: Some(name),
                id: parent.cast(),
            });
        }

        if let Some(old_child) = old_child.into() {
            debug!("Old child {:?}", old_child);
            let new = &mut slab[new_id];
            new.next = OptionId::some(old_child);
            if parent_dead {
                debug!("Parent already dead, setting liveness appropriately.");
                new.liveness = Liveness::DeadDueToParent;
                removed.push(new_id);
            }
            slab[old_child].previous = OptionId::some(new_id);
        } else if parent_dead {
            debug!("No previous child, but parent is already dead.");
            slab[new_id].liveness = Liveness::DeadDueToParent;
        } else {
            debug!("No previous child.");
        }
        debug!(
            "Added entity {:?} {:?} as child of {:?}...",
            name, new_id, parent
        );
        Ok(new_id)
    }

    pub fn remove(&mut self, id: EntityId) {
        debug!("Lazily removed entity {:?}...", id);
        self.removed.push(id);
    }

    #[inline]
    pub fn last_removed(&self) -> &[EntityId] {
        &self.last_removed
    }

    #[inline]
    pub fn get(&self, id: EntityId) -> Option<&Entity> {
        self.slab.get(id)
    }

    #[inline]
    pub fn debug_name_of(&self, id: EntityId) -> Option<&'static str> {
        self.slab.get(id).map(|entity| entity.name)
    }

    pub fn debug_tree_dump(&self, indent: usize) -> String {
        let mut output = "Entity tree dump:\n".to_owned();
        let mut stack = Vec::new();
        stack.push((
            0,
            if let Some(root) = self.first_root.into_option() {
                root
            } else {
                return output;
            },
        ));

        while let Some((depth, id)) = stack.pop() {
            for _ in 0..(indent + depth * 4) {
                output.push(' ');
            }

            if let Some(entity) = self.slab.get(id) {
                write!(&mut output, "|- {}  ", entity.name).expect("string write fail");
                let id_padding = {
                    let length = indent + depth * 4 + 3 + entity.name.len() + 4;
                    if length > 60 {
                        0
                    } else {
                        60 - length
                    }
                };
                for _ in 0..id_padding {
                    output.push('.');
                }
                writeln!(&mut output, "  ({:?})", id).expect("string write fail");
                if let Some(next_id) = entity.next.into_option() {
                    stack.push((depth, next_id));
                }

                if let Some(child_id) = entity.child.into_option() {
                    stack.push((depth + 1, child_id));
                }
            } else {
                output.push_str("|- <missing>\n");
            };
        }

        output
    }
}

impl<'context> InfallibleSystem<'context> for Entities {
    type Dependencies = ();

    fn debug_name() -> &'static str {
        "entities"
    }

    fn create(_deps: ()) -> Self {
        Self {
            slab: IdSlab::with_capacity(1024),
            first_root: OptionId::none(),
            removed: Vec::with_capacity(1024),
            last_removed: Vec::with_capacity(1024),
        }
    }

    // TODO(cristicbz): Split up into simpler, more self-documenting functions.
    #[cfg_attr(feature = "cargo-clippy", allow(clippy::cyclomatic_complexity))]
    fn update(&mut self, _dependencies: ()) {
        let Self {
            ref mut removed,
            ref mut last_removed,
            ref mut slab,
            ref mut first_root,
            ..
        } = *self;
        if removed.is_empty() {
            return;
        }
        let num_killed_in_removed = removed.len();
        debug!(
            "Collecting removed. Explictly removed {} ids.",
            num_killed_in_removed
        );

        // First iterate through the explictly killed deleted entities. As we go through them we
        //   (A) If the entity is marked as killed or orphan then skip.
        //   (B) Otherwise mark as killed (this will make sure we dedupe the killed entities).
        //   (C) Push entity to `last_removed`.
        //   (D) Push its first child to `removed`.
        last_removed.clear();
        for i_removed in 0..num_killed_in_removed {
            let removed_id = removed[i_removed];
            let &mut Entity {
                child,
                name,
                ref mut liveness,
                ..
            } = if let Some(entity) = slab.get_mut(removed_id) {
                entity
            } else {
                debug!("Skipping already removed {:?}.", removed_id);
                continue;
            };
            if !liveness.is_alive() {
                debug!(
                    "Explicitly removed {:?} ({:?}) was already processed.",
                    name, removed_id
                );
                continue;
            }

            *liveness = Liveness::Killed;
            last_removed.push(removed_id);
            if let Some(child) = child.into_option() {
                debug!(
                    "Adding child {:?} of {:?} ({:?}) to orphan queue.",
                    child, name, removed_id
                );
                removed.push(child);
            }
        }
        let num_killed_in_last_removed = last_removed.len();
        debug!(
            "Deduplicated explictly removed {} ids.",
            num_killed_in_last_removed
        );

        let mut i_removed = num_killed_in_removed;
        while i_removed < removed.len() {
            let mut removed_id = removed[i_removed];
            loop {
                let Entity {
                    liveness,
                    next,
                    child,
                    name,
                    ..
                } = slab.remove(removed_id).expect("missing removed child");
                debug!("Removed orphan entity {:?} {:?}.", name, removed_id);

                if liveness.is_alive() {
                    last_removed.push(removed_id);
                    if let Some(child) = child.into() {
                        debug!(
                            "Added child for {:?} ({:?}) to queue: {:?}",
                            name, removed_id, child
                        );
                        removed.push(child);
                    } else {
                        debug!(
                            "Entity {:?} ({:?}) was alive but had no children to remove.",
                            name, removed_id,
                        );
                    }
                } else {
                    debug!(
                        "Entity {:?} ({:?}) was already marked as {:?}, skipping.",
                        name, removed_id, liveness
                    );
                }

                if let Some(sibling) = next.into() {
                    debug!("Moving to sibling {:?}", sibling);
                    removed_id = sibling;
                } else {
                    debug!("No more siblings.");
                    break;
                }
            }
            i_removed += 1;
        }

        for &removed_id in &last_removed[..num_killed_in_last_removed] {
            let Entity {
                next,
                previous,
                parent,
                name,
                ..
            } = if let Some(entity) = slab.remove(removed_id) {
                entity
            } else {
                // Removed by the orphan processing loop.
                debug!("Skipped already removed {:?}.", removed_id);
                continue;
            };

            debug!("Removed killed {:?} ({:?})", name, removed_id);
            if let Some((next_id, next)) = next.into_option().map(|id| (id, &mut slab[id])) {
                debug!(
                    "Patched previous for {:?} ({:?}) to point to {:?}",
                    next.name, next_id, previous
                );
                next.previous = previous;
            }

            if let Some((previous_id, previous)) =
                previous.into_option().map(|id| (id, &mut slab[id]))
            {
                debug!(
                    "Patched next for {:?} ({:?}) to point to {:?}",
                    previous.name, previous_id, next
                );
                previous.next = next;
            }

            if let Some(parent_id) = parent.into_option() {
                let parent = &mut slab[parent_id];
                if parent.child.into_option().expect("parent has no children") == removed_id {
                    debug!("Patched child for {:?} to point to {:?}", parent.name, next);
                    parent.child = next;
                }
            } else if first_root.expect("no root") == removed_id {
                debug!("Patched first root to point to {:?}", next);
                *first_root = next
            }
        }

        debug!("Collected {:?} removed ids.", last_removed.len());
        removed.clear();
    }

    fn teardown(&mut self, _deps: ()) {
        self.update(());
    }

    fn destroy(mut self, _deps: ()) {
        self.update(());
        if !self.is_empty() {
            error!("Entities leaked. {}", self.debug_tree_dump(4));
        }
    }
}

#[derive(Eq, PartialEq, Debug)]
enum Liveness {
    Alive,
    Killed,
    DeadDueToParent,
}

impl Liveness {
    fn is_alive(&self) -> bool {
        *self == Liveness::Alive
    }
}

pub struct Entity {
    name: &'static str,
    parent: OptionId<Entity>,
    child: OptionId<Entity>,

    next: OptionId<Entity>,
    previous: OptionId<Entity>,

    liveness: Liveness,
}

impl Entity {
    #[inline]
    pub fn parent(&self) -> Option<EntityId> {
        self.parent.into_option()
    }
}

#[cfg(test)]
mod test {
    use super::super::system::InfallibleSystem;
    use super::{Entities, EntityId};
    use std::collections::HashSet;

    struct Tree1 {
        root_a: EntityId,
        root_b: EntityId,
        root_c: EntityId,

        a1: EntityId,

        a2: EntityId,
        a2x: EntityId,
        a2xa: EntityId,
        a2xb: EntityId,

        c1: EntityId,

        a2y: EntityId,
    }

    impl Tree1 {
        fn new(entities: &mut Entities) -> Self {
            let root_a = entities.add_root("root_a");
            let root_b = entities.add_root("root_b");
            let root_c = entities.add_root("root_c");

            let a1 = entities.add(root_a, "a1").unwrap();

            let a2 = entities.add(root_a, "a2").unwrap();
            let a2x = entities.add(a2, "a2x").unwrap();
            let a2xa = entities.add(a2x, "a2xa").unwrap();
            let a2xb = entities.add(a2x, "a2xb").unwrap();

            let c1 = entities.add(root_c, "c1").unwrap();

            let a2y = entities.add(a2, "a2y").unwrap();
            Self {
                root_a,
                root_b,
                root_c,

                a1,

                a2,
                a2x,
                a2xa,
                a2xb,

                c1,

                a2y,
            }
        }
    }

    fn check_removed(entities: &Entities, expected: &[EntityId]) {
        let actual: HashSet<EntityId> = entities
            .last_removed
            .iter()
            .cloned()
            .collect::<HashSet<_>>();
        assert!(
            expected.len() == actual.len() && expected.iter().all(|id| actual.contains(id)),
            "actual: {:?}\nexpected: {:?}",
            entities.removed,
            expected
        );
    }

    #[test]
    fn add_contains() {
        let mut entities = Entities::create(());
        let tree1 = Tree1::new(&mut entities);

        assert!(entities.contains(tree1.root_a));
        assert!(entities.contains(tree1.root_b));
        assert!(entities.contains(tree1.root_c));
        assert!(entities.contains(tree1.a1));
        assert!(entities.contains(tree1.a2));
        assert!(entities.contains(tree1.a2x));
        assert!(entities.contains(tree1.a2xa));
        assert!(entities.contains(tree1.a2xb));
        assert!(entities.contains(tree1.c1));
        assert!(entities.contains(tree1.a2y));

        assert_eq!(entities.len(), 10);
        assert_eq!(entities.removed.len(), 0);
    }

    #[test]
    fn add_remove_single() {
        let mut entities = Entities::create(());
        let tree1 = Tree1::new(&mut entities);

        entities.remove(tree1.root_b);
        assert_eq!(&entities.removed, &[tree1.root_b]);
        entities.update(());
        assert_eq!(entities.last_removed, &[tree1.root_b]);

        assert_eq!(entities.removed.len(), 0);
        assert!(!entities.contains(tree1.root_b));
    }

    #[test]
    fn add_remove_one_child() {
        let mut entities = Entities::create(());
        let tree1 = Tree1::new(&mut entities);

        entities.remove(tree1.root_c);
        entities.update(());
        check_removed(&entities, &[tree1.c1, tree1.root_c]);

        assert_eq!(entities.removed.len(), 0);
        assert!(!entities.contains(tree1.c1));
        assert!(!entities.contains(tree1.root_c));
    }

    #[test]
    fn add_remove_one_subtree() {
        let mut entities = Entities::create(());
        let tree1 = Tree1::new(&mut entities);

        entities.remove(tree1.a2x);
        entities.update(());
        check_removed(&entities, &[tree1.a2xa, tree1.a2xb, tree1.a2x]);

        assert_eq!(entities.removed.len(), 0);
        assert!(!entities.contains(tree1.a2xa));
        assert!(!entities.contains(tree1.a2xb));
        assert!(!entities.contains(tree1.a2x));

        assert!(entities.contains(tree1.a2y));
        assert!(entities.contains(tree1.a2));
        assert!(entities.contains(tree1.root_a));
    }

    #[test]
    fn add_remove_all() {
        let mut entities = Entities::create(());
        let tree1 = Tree1::new(&mut entities);

        entities.remove(tree1.a2y);
        entities.update(());
        assert_eq!(entities.last_removed, &[tree1.a2y]);
        assert_eq!(entities.removed.len(), 0);

        entities.remove(tree1.a2);
        entities.remove(tree1.root_a);
        entities.remove(tree1.root_c);

        let c2 = entities.add(tree1.root_c, "c2").unwrap();
        entities.update(());
        check_removed(
            &entities,
            &[
                tree1.a2xa,
                tree1.a2xb,
                tree1.a2x,
                tree1.a2,
                tree1.a1,
                tree1.root_a,
                tree1.c1,
                tree1.root_c,
                c2,
            ],
        );

        entities.remove(tree1.root_b);
        entities.update(());

        assert_eq!(entities.len(), 0);
    }
}
