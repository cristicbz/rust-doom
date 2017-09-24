use super::errors::{ErrorKind, Result};
use super::system::InfallibleSystem;
use idcontain::{IdSlab, Id, OptionId};
use std::collections::VecDeque;
use std::fmt::Write;
use std::mem;

pub type EntityId = Id<Entity>;

pub struct Entities {
    slab: IdSlab<Entity>,
    first_root: OptionId<Entity>,
    removed: Vec<EntityId>,
    last_removed: Vec<EntityId>,
    removed_children: VecDeque<EntityId>,
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
        let old_first_root: Option<EntityId> = mem::replace(first_root, OptionId::some(new_id))
            .into();
        if let Some(old_first_root) = old_first_root {
            let old_entity = &mut slab[old_first_root];
            debug!(
                "Patched previous of root {:?} {:?} to {:?}...",
                old_entity.name,
                old_first_root,
                new_id
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
            bail!(ErrorKind::NoSuchEntity("add", Some(name), parent.cast()));
        }

        if let Some(old_child) = old_child.into() {
            debug!("Old child {:?}", old_child);
            {
                let new = &mut slab[new_id];
                new.next = OptionId::some(old_child);
                if parent_dead {
                    debug!("Parent already dead, setting liveness appropriately.");
                    new.liveness = Liveness::DeadDueToParent;
                    removed.push(new_id);
                }
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
            name,
            new_id,
            parent
        );
        Ok(new_id)
    }

    pub fn remove(&mut self, mut id: EntityId) -> Result<()> {
        let Entities {
            ref mut slab,
            ref mut removed,
            ref mut removed_children,
            ..
        } = *self;
        debug!("Removing {:?}...", id);
        let removed_start = removed.len();
        id = {
            let this = slab.get_mut(id).ok_or_else(|| {
                debug!("Entity not in slab, quitting early.");
                ErrorKind::NoSuchEntity("remove", None, id.cast())
            })?;
            if !this.liveness.is_alive() {
                debug!(
                    "Entity {:?} is in slab, but already deleted: {:?}",
                    this.name,
                    this.liveness
                );
                return Ok(());
            }
            this.liveness = Liveness::Killed;
            removed.push(id);
            if let Some(child) = this.child.into() {
                debug!(
                    "Killed entity {:?} and added child {:?} to queue.",
                    this.name,
                    child
                );
                child
            } else {
                debug!("Killed entity {:?} with no children.", this.name);
                return Ok(());
            }
        };

        loop {
            loop {
                let this = &mut slab[id];
                if mem::replace(&mut this.liveness, Liveness::DeadDueToParent).is_alive() {
                    removed.push(id);
                    debug!("Killed entity {:?} {:?}.", this.name, id);
                    if let Some(child) = this.child.into() {
                        debug!(
                            "Added child for {:?} {:?} to queue: {:?}",
                            this.name,
                            id,
                            child
                        );
                        removed_children.push_back(child);
                    }
                } else {
                    debug!("Entity {:?} {:?} already dead, skipping.", this.name, id);
                }
                if let Some(sibling) = this.next.into() {
                    debug!("Moving to sibling {:?}", sibling);
                    id = sibling;
                } else {
                    debug!("No more siblings.");
                    break;
                }
            }
            id = if let Some(id) = removed_children.pop_front() {
                debug!("Popped child {:?} off queue.", id);
                id
            } else {
                debug!("No more children.");
                break;
            };
        }

        debug!(
            "Killed {} entities: {:?}",
            removed.len() - removed_start,
            &removed[removed_start..],
            );

        Ok(())
    }

    #[inline]
    pub fn last_removed(&self) -> &[EntityId] {
        &self.last_removed
    }

    #[inline]
    pub fn parent_of(&self, id: EntityId) -> Result<Option<EntityId>> {
        if let Some(entity) = self.slab.get(id) {
            Ok(entity.parent.into_option())
        } else {
            bail!(ErrorKind::NoSuchEntity("parent_of", None, id.cast()));
        }
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
                    if length > 60 { 0 } else { 60 - length }
                };
                for _ in 0..id_padding {
                    output.push('.');
                }
                write!(&mut output, "  ({:?})\n", id).expect("string write fail");
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
        Entities {
            slab: IdSlab::with_capacity(1024),
            first_root: OptionId::none(),
            removed: Vec::with_capacity(1024),
            last_removed: Vec::with_capacity(1024),
            removed_children: VecDeque::with_capacity(1024),
        }
    }

    fn update(&mut self, _dependencies: ()) {
        let Entities {
            ref mut removed,
            ref mut last_removed,
            ref mut slab,
            ref mut first_root,
            ..
        } = *self;
        if removed.is_empty() {
            return;
        }
        debug!("Collecting removed, {} ids.", removed.len());
        for &removed_id in &*removed {
            if let Some(Entity {
                            next,
                            previous,
                            parent,
                            liveness,
                            name,
                            ..
                        }) = slab.remove(removed_id)
            {
                debug!("Collecting {:?} {:?}", name, removed_id);
                // If the entitiy is dead via its parent then all its siblings (and its parent) are
                // dead, so there's not point fixing up sibling and parent pointers.
                if liveness.is_dead_due_to_parent() {
                    debug!("Dead due to parent, no patching required.");
                    continue;
                }
                assert!(liveness.is_killed(), "{:?} {:?}", name, removed_id);
                if let Some(next) = next.into_option().and_then(|id| slab.get_mut(id)) {
                    debug!(
                        "Patched previous for {:?} to point to {:?}",
                        next.name,
                        previous
                    );
                    next.previous = previous;
                }

                if let Some(previous) = previous.into_option().and_then(|id| slab.get_mut(id)) {
                    debug!(
                        "Patched next for {:?} to point to {:?}",
                        previous.name,
                        next
                    );
                    previous.next = next;
                }

                if let Some(parent_id) = parent.into_option() {
                    if let Some(parent) = slab.get_mut(parent_id) {
                        if parent.child.map_or(false, |id| id == removed_id) {
                            debug!("Patched child for {:?} to point to {:?}", parent.name, next);
                            parent.child = next;
                        }
                    }
                } else if first_root.map_or(false, |id| id == removed_id) {
                    debug!("Patched first root to point to {:?}", next);
                    *first_root = next
                }
            } else {
                error!("Garbage {:?} was already collected.", removed_id);
            }
        }
        debug!("Collected {:?} removed ids.", removed.len());
        mem::swap(removed, last_removed);
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

    fn is_killed(&self) -> bool {
        *self == Liveness::Killed
    }

    fn is_dead_due_to_parent(&self) -> bool {
        *self == Liveness::DeadDueToParent
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


#[cfg(test)]
mod test {
    use super::{Entities, EntityId};
    use super::super::system::InfallibleSystem;
    use env_logger;
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
            Tree1 {
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
        let actual: HashSet<EntityId> = entities.removed.iter().cloned().collect::<HashSet<_>>();
        assert!(
            expected.len() == actual.len() && expected.iter().all(|id| actual.contains(id)),
            "actual: {:?}\nexpected: {:?}",
            entities.removed,
            expected
        );
    }

    #[test]
    fn add_contains() {
        let _ = env_logger::init();
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
        let _ = env_logger::init();
        let mut entities = Entities::create(());
        let tree1 = Tree1::new(&mut entities);

        entities.remove(tree1.root_b).unwrap();
        assert_eq!(&entities.removed, &[tree1.root_b]);
        entities.update(());
        assert_eq!(entities.last_removed, &[tree1.root_b]);

        assert_eq!(entities.removed.len(), 0);
        assert!(!entities.contains(tree1.root_b));
    }

    #[test]
    fn add_remove_one_child() {
        let _ = env_logger::init();
        let mut entities = Entities::create(());
        let tree1 = Tree1::new(&mut entities);

        entities.remove(tree1.root_c).unwrap();
        check_removed(&entities, &[tree1.c1, tree1.root_c]);
        entities.update(());

        assert_eq!(entities.removed.len(), 0);
        assert!(!entities.contains(tree1.c1));
        assert!(!entities.contains(tree1.root_c));
    }

    #[test]
    fn add_remove_one_subtree() {
        let _ = env_logger::init();
        let mut entities = Entities::create(());
        let tree1 = Tree1::new(&mut entities);

        entities.remove(tree1.a2x).unwrap();
        check_removed(&entities, &[tree1.a2xa, tree1.a2xb, tree1.a2x]);
        entities.update(());

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
        let _ = env_logger::init();
        let mut entities = Entities::create(());
        let tree1 = Tree1::new(&mut entities);

        entities.remove(tree1.a2y).unwrap();
        assert_eq!(entities.removed, &[tree1.a2y]);
        entities.update(());
        assert_eq!(entities.removed.len(), 0);

        entities.remove(tree1.a2).unwrap();
        check_removed(&entities, &[tree1.a2xa, tree1.a2xb, tree1.a2x, tree1.a2]);

        entities.remove(tree1.root_a).unwrap();
        check_removed(
            &entities,
            &[
                tree1.a2xa,
                tree1.a2xb,
                tree1.a2x,
                tree1.a2,
                tree1.a1,
                tree1.root_a,
            ],
        );

        entities.remove(tree1.root_c).unwrap();
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
            ],
        );

        let c2 = entities.add(tree1.root_c, "c2").unwrap();
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

        entities.update(());
        entities.remove(tree1.root_b).unwrap();
        entities.update(());

        assert_eq!(entities.len(), 0);
    }
}
