use shipyard::*;

/// # Update one to one
///
/// A shipyard view for when you need to assign some component
/// based upon another component.
///
/// This is called "one-to-one", because you cannot optionally assign the
/// "write" component. If there is a "read" component changed, the "write"
/// component must have a value to be assigned.
///
/// Automatically manages checking for distinct values (if the "write" component
/// == its previous value, then no update). And, this view manages removing the
/// "write" component if the "read" component is removed.
///
/// ```
/// use shipyard_app::prelude::*;
///
/// /// Square is a one-to-one value determined from the [u32] components changes.
/// #[derive(Clone, Debug, PartialEq)]
/// struct Square(u64);
///
/// fn updating_squares(u32_to_square: UpdateOneToOne<u32, Square>) {
///     u32_to_square.update(|_entity_id, u32_component| {
///         Square((*u32_component as u64) * (*u32_component as u64))
///     });
/// }
///
/// fn collecting_squares(v_u32: View<u32>, v_square: View<Square>) -> Vec<(u32, Square)> {
///     let mut res = (&v_u32, &v_square).iter().map(|(u, s)| (u.clone(), s.clone())).collect::<Vec<(u32, Square)>>();
///     res.sort_unstable_by_key(|(u, _)| *u);
///     res
/// }
///
/// let mut world = World::new();
/// world.run(|mut vm_u32: ViewMut<u32>| {
///     vm_u32.update_pack();
/// }).unwrap();
///
/// let entity_1 = world.add_entity((1u32,));
/// world.add_entity((2u32,));
/// world.add_entity((3u32,));
///
/// world.run(updating_squares).unwrap();
///
/// assert_eq!(
///     world.run(collecting_squares).unwrap(),
///     vec![(1u32, Square(1)), (2u32, Square(4)), (3u32, Square(9))]
/// );
///
/// // remove
/// world.run(|mut vm_u32: ViewMut<u32>| {
///     vm_u32.remove(entity_1).unwrap();
/// }).unwrap();
///
/// world.run(updating_squares).unwrap();
///
/// assert_eq!(
///     world.run(collecting_squares).unwrap(),
///     vec![(2u32, Square(4)), (3u32, Square(9))]
/// );
///
/// // entity_1 was removed from the [Square] storage as well
/// world.borrow::<View<Square>>()
///     .unwrap()
///     .get(entity_1)
///     .expect_err("expect Square removed in one to one");
/// ```
pub struct UpdateOneToOne<'a, T, U: PartialEq>(View<'a, T>, ViewMut<'a, U>);

impl<'a, T: 'static + Send + Sync, U: 'static + Send + Sync + PartialEq> Borrow<'a>
    for UpdateOneToOne<'a, T, U>
{
    fn borrow(
        all_storages: &'a AllStorages,
        all_borrow: Option<SharedBorrow<'a>>,
    ) -> Result<Self, error::GetStorage>
    where
        Self: Sized,
    {
        let (v_t, vm_u): (View<'a, T>, ViewMut<'a, U>) = Borrow::borrow(all_storages, all_borrow)?;
        Ok(UpdateOneToOne(v_t, vm_u))
    }
}

unsafe impl<'a, T: 'static + Send + Sync, U: 'static + Send + Sync + PartialEq> BorrowInfo
    for UpdateOneToOne<'a, T, U>
{
    fn borrow_info(mut info: &mut Vec<info::TypeInfo>) {
        View::<'a, T>::borrow_info(&mut info);
        ViewMut::<'a, U>::borrow_info(&mut info);
    }
}

impl<'a, T, U> UpdateOneToOne<'a, T, U>
where
    T: Sync + Send + 'static,
    U: PartialEq + Sync + Send + 'static,
{
    pub fn update<F>(self, mut update_fn: F)
    where
        F: FnMut(EntityId, &T) -> U,
    {
        self.update_or_ignore(move |e, t| Some(update_fn(e, t)))
    }

    pub fn update_or_ignore<F>(self, mut update_fn: F)
    where
        F: FnMut(EntityId, &T) -> Option<U>,
    {
        let UpdateOneToOne(v_t, mut vm_u) = self;
        for (e, t) in (&v_t).inserted().iter().with_id() {
            if let Some(update) = update_fn(e, t) {
                vm_u.add_component_unchecked(e, update)
            }
        }
        for (e, t) in (&v_t).modified().iter().with_id() {
            if let Some(update) = update_fn(e, t) {
                if let Ok(ref mut exist) = (&mut vm_u).get(e) {
                    if !exist.eq(&update) {
                        *exist.as_mut() = update;
                    }
                } else {
                    vm_u.add_component_unchecked(e, update);
                }
            }
        }
        for e in (&v_t).removed_or_deleted() {
            vm_u.delete(e);
        }
    }

    pub fn update_or_delete<F>(self, mut update_fn: F)
    where
        F: FnMut(EntityId, &T) -> Option<U>,
    {
        let UpdateOneToOne(v_t, mut vm_u) = self;
        for (e, t) in (&v_t).inserted().iter().with_id() {
            if let Some(update) = update_fn(e, t) {
                vm_u.add_component_unchecked(e, update)
            } else {
                vm_u.delete(e);
            }
        }
        for (e, t) in (&v_t).modified().iter().with_id() {
            if let Some(update) = update_fn(e, t) {
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
        for e in (&v_t).removed_or_deleted() {
            vm_u.delete(e);
        }
    }
}
