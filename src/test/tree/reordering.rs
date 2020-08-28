use super::*;

#[derive(Clone)]
pub struct MoveCmd {
    pub target: EntityId,
    pub place: MoveToPlace,
}

#[derive(Clone)]
pub enum MoveToPlace {
    Unlink,
    Between(EntityId, EntityId),
    FirstChildOf(EntityId),
    LastChildOf(EntityId),
}

pub fn tree_reordering(
    (mut commands, mut vm_child_of, v_parent_index): (
        UniqueMoveCommands,
        ViewMut<ChildOf>,
        View<ParentIndex>,
    ),
) {
    for cmd in commands.drain() {
        *(&mut vm_child_of).get(cmd.target).unwrap() = match cmd.place {
            MoveToPlace::Between(a, b) => {
                // check that a & b are both of the same parent
                let ChildOf(a_of, a_ord) = (&vm_child_of).get(a).unwrap();
                let ChildOf(b_of, b_ord) = (&vm_child_of).get(b).unwrap();

                if a_of != b_of {
                    panic!("MoveToPlace::Between: failed to reorder between targets two elements of different parents target={:?}; {:?} vs {:?}", cmd.target, a_of, b_of);
                }

                // set value to
                ChildOf(*a_of, Ordered::between(&a_ord, &b_ord))
            }
            MoveToPlace::FirstChildOf(parent) => v_parent_index
                .get(parent)
                .ok()
                .and_then(|parent_index: &ParentIndex| parent_index.children.first())
                .map(|first_child| ChildOf(parent, Ordered::before(&first_child.0)))
                // found no first child in index, create new ChildOf
                .unwrap_or_else(|| ChildOf(parent, Ordered::hinted(0))),
            MoveToPlace::LastChildOf(parent) => v_parent_index
                .get(parent)
                .ok()
                .and_then(|parent_index: &ParentIndex| parent_index.children.last())
                .map(|last_child| ChildOf(parent, Ordered::after(&last_child.0)))
                // found no last child in index, create new ChildOf
                .unwrap_or_else(|| ChildOf(parent, Ordered::hinted(0))),
            MoveToPlace::Unlink => ChildOf(EntityId::dead(), Ordered::hinted(0)),
        }
    }
}
