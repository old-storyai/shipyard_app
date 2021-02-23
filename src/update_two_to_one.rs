use std::collections::HashSet;

use shipyard::*;

/// # Update two to one
///
/// A shipyard view for when you need to assign some component
/// based upon another component.
///
/// This is called "two-to-one", because you cannot optionally assign the
/// "write" component. If there is a "read" component changed, the "write"
/// component must have a value to be assigned.
///
/// Automatically manages checking for distinct values (if the "write" component
/// == its previous value, then no update). And, this view manages removing the
/// "write" component if the "read" component is removed.
pub struct UpdateTwoToOne<'a, T1, T2, U: PartialEq>(View<'a, T1>, View<'a, T2>, ViewMut<'a, U>);

impl<'a, T1, T2, U> Borrow<'a> for UpdateTwoToOne<'a, T1, T2, U>
where
    T1: Sync + Send + 'static,
    T2: Sync + Send + 'static,
    U: PartialEq + Sync + Send + 'static,
{
    fn borrow(
        all_storages: &'a AllStorages,
        all_borrow: Option<SharedBorrow<'a>>,
    ) -> Result<Self, error::GetStorage>
    where
        Self: Sized,
    {
        let (v_t1, v_t2, vm_u): (View<'a, T1>, View<'a, T2>, ViewMut<'a, U>) =
            Borrow::borrow(all_storages, all_borrow)?;
        Ok(UpdateTwoToOne(v_t1, v_t2, vm_u))
    }
}

unsafe impl<'a, T1, T2, U> BorrowInfo for UpdateTwoToOne<'a, T1, T2, U>
where
    T1: Sync + Send + 'static,
    T2: Sync + Send + 'static,
    U: PartialEq + Sync + Send + 'static,
{
    fn borrow_info(mut info: &mut Vec<info::TypeInfo>) {
        View::<'a, T1>::borrow_info(&mut info);
        View::<'a, T2>::borrow_info(&mut info);
        ViewMut::<'a, U>::borrow_info(&mut info);
    }
}

impl<'a, T1, T2, U> UpdateTwoToOne<'a, T1, T2, U>
where
    T1: Sync + Send + 'static,
    T2: Sync + Send + 'static,
    U: PartialEq + Sync + Send + 'static,
{
    /// Delete if either component is not present or return `None`
    #[track_caller]
    pub fn update_or_delete<F>(self, mut update_fn: F)
    where
        F: FnMut(EntityId, &T1, &T2) -> Option<U>,
    {
        let UpdateTwoToOne(v_t1, v_t2, mut vm_u) = self;

        let mut deleted_ids = HashSet::new();
        deleted_ids.extend((&v_t1).removed_or_deleted());
        deleted_ids.extend((&v_t2).removed_or_deleted());
        for e in deleted_ids.iter().copied() {
            vm_u.delete(e);
        }

        let mut handled_ids = deleted_ids;

        let mut inserted_ids = HashSet::new();
        inserted_ids.extend(
            (&v_t1)
                .inserted()
                .iter()
                .ids()
                .filter(|e| !handled_ids.contains(e)),
        );
        inserted_ids.extend(
            (&v_t2)
                .inserted()
                .iter()
                .ids()
                .filter(|e| !handled_ids.contains(e)),
        );

        for e in inserted_ids.iter().copied() {
            if let Ok((t1, t2)) = (&v_t1, &v_t2).get(e) {
                if let Some(update) = update_fn(e, t1, t2) {
                    vm_u.add_component_unchecked(e, update)
                } else {
                    vm_u.delete(e);
                }
            }
        }

        handled_ids.extend(inserted_ids);

        let mut modified_ids = HashSet::new();
        modified_ids.extend(
            (&v_t1)
                .modified()
                .iter()
                .ids()
                .filter(|e| !handled_ids.contains(e)),
        );
        modified_ids.extend(
            (&v_t2)
                .modified()
                .iter()
                .ids()
                .filter(|e| !handled_ids.contains(e)),
        );
        for e in modified_ids {
            if let Ok((t1, t2)) = (&v_t1, &v_t2).get(e) {
                if let Some(update) = update_fn(e, t1, t2) {
                    if let Ok(ref mut exist) = (&mut vm_u).get(e) {
                        if !exist.eq(&update) {
                            *exist.as_mut() = update;
                        }
                    } else {
                        vm_u.add_component_unchecked(e, update);
                    }
                } else {
                    vm_u.delete(e);
                }
            }
        }
    }
}
