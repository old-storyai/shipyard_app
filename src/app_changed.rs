use crate::prelude::*;

struct Changed1<'a, A>(View<'a, A>);
// struct Changed2<'a, A, B>(View<'a, A>, View<'a, B>);
// struct Changed3<'a, A, B, C>(View<'a, A>, View<'a, B>, View<'a, C>);

impl<'a, A: 'static> Changed<'a, &'a A> for Changed1<'a, A> {
    fn changed(&'a self) -> ChangedEntities<&'a A> {
        ChangedEntities {
            inserted: self.0.inserted().iter().with_id().collect(),
            modified: self.0.modified().iter().with_id().collect(),
            deleted_or_removed: self.0.removed_or_deleted().collect(),
        }
    }
}

// impl<'a, A: 'static, B: 'static> Changed<'a, (&'a A, &'a B)> for Changed2<'a, A, B> {
//     fn changed(&'a self) -> ChangedEntities<(&'a A, &'a B)> {
//         ChangedEntities {
//             inserted: self.0.inserted().iter().with_id().collect(),
//             modified: self.0.modified().iter().with_id().collect(),
//             deleted_or_removed: self.0.removed_or_deleted().collect(),
//         }
//     }
// }

struct ChangedEntities<T> {
    deleted_or_removed: Vec<EntityId>,
    inserted: Vec<(EntityId, T)>,
    modified: Vec<(EntityId, T)>,
}

trait Changed<'a, T> {
    fn changed(&'a self) -> ChangedEntities<T>;
}

impl<'a, T> Borrow<'a> for Changed1<'a, T>
where
    T: Sync + Send + 'static,
{
    fn try_borrow(world: &'a World) -> Result<Self, error::GetStorage>
    where
        Self: Sized,
    {
        Ok(Changed1(world.borrow::<View<'a, T>>()?))
    }

    fn borrow_info(mut info: &mut Vec<info::TypeInfo>) {
        View::<'a, T>::borrow_info(&mut info);
    }
}

pub struct ChangedOneToOne<'a, T, U>(View<'a, T>, ViewMut<'a, U>);

impl<'a, T, U> Borrow<'a> for ChangedOneToOne<'a, T, U>
where
    T: Sync + Send + 'static,
    U: Sync + Send + 'static,
{
    fn try_borrow(world: &'a World) -> Result<Self, error::GetStorage>
    where
        Self: Sized,
    {
        Ok(ChangedOneToOne(
            world.borrow::<View<'a, T>>()?,
            world.borrow::<ViewMut<'a, U>>()?,
        ))
    }

    fn borrow_info(mut info: &mut Vec<info::TypeInfo>) {
        View::<'a, T>::borrow_info(&mut info);
        ViewMut::<'a, U>::borrow_info(&mut info);
    }
}

impl<'a, T, U> ChangedOneToOne<'a, T, U>
where
    T: Sync + Send + 'static,
    U: PartialEq + Sync + Send + 'static,
{
    pub fn update<'b: 'a, F>(&'a mut self, mut update_fn: F)
    where
        F: 'static + FnMut(EntityId, &'a T) -> U,
    {
        let v_t = &self.0;
        let vm_u = &mut self.1;
        for (e, t) in v_t.inserted().iter().with_id() {
            vm_u.add_component_unchecked(e, update_fn(e, t))
        }
        for (e, t) in v_t.modified().iter().with_id() {
            if let Ok(ref mut exist) = vm_u.get(e) {
                let update = update_fn(e, t);
                if !exist.eq(&update) {
                    *exist.as_mut() = update;
                }
            } else {
                vm_u.add_component_unchecked(e, update_fn(e, t));
            }
        }
        for e in v_t.removed_or_deleted() {
            vm_u.delete(e);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    struct A(pub usize);
    #[derive(PartialEq)]
    struct APlusOne(pub usize);
    // struct B(pub usize);
    // struct C(pub usize);
    // struct D(pub usize);

    fn sys_a_add_1(entities: EntitiesView, mut update_a_plus_one: ChangedOneToOne<A, APlusOne>) {
        update_a_plus_one.update(|_, prev| APlusOne(prev.0 + 1));
    }

    #[test]
    fn creates_cycle_for_one() {
        // let
    }
}
