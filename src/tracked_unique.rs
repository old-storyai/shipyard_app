//! A small change tracker to wrap around a unique so you can determine if the value changes between updates.
use tracing::trace_span;

use crate::prelude::*;

use core::any::type_name;
use std::{fmt, ops::Deref, ops::DerefMut};

pub struct TrackedValue<T: 'static>(InnerTrackedState, T);

pub struct Tracked<'a, T: 'static>(UniqueView<'a, TrackedValue<T>>);

pub struct TrackedBorrower<T>(T);

impl<T: 'static + Send + Sync> IntoBorrow for Tracked<'_, T> {
    type Borrow = TrackedBorrower<T>;
}

impl<'a, T: 'static + Send + Sync> Borrow<'a> for TrackedBorrower<T> {
    type View = Tracked<'a, T>;

    fn borrow(world: &'a World) -> Result<Self::View, error::GetStorage>
    where
        Self: Sized,
    {
        Ok(Tracked(world.borrow()?))
    }
}

impl<'a, T: 'static + Send + Sync> AllStoragesBorrow<'a> for TrackedBorrower<T> {
    fn all_borrow(all_storages: &'a AllStorages) -> Result<Self::View, error::GetStorage>
    where
        Self: Sized,
    {
        Ok(Tracked(all_storages.borrow()?))
    }
}

unsafe impl<'a, T: 'static + Send + Sync> BorrowInfo for Tracked<'a, T> {
    fn borrow_info(mut info: &mut Vec<info::TypeInfo>) {
        UniqueView::<'a, T>::borrow_info(&mut info);
    }
}

pub struct TrackedMut<'a, T: 'static>(UniqueViewMut<'a, TrackedValue<T>>);

pub struct TrackedMutBorrower<T: 'static>(T);

impl<'a, T: 'static + Send + Sync> IntoBorrow for TrackedMut<'a, T> {
    type Borrow = TrackedMutBorrower<T>;
}

impl<'a, T: 'static + Send + Sync> Borrow<'a> for TrackedMutBorrower<T> {
    type View = TrackedMut<'a, T>;

    fn borrow(world: &'a World) -> Result<Self::View, error::GetStorage>
    where
        Self: Sized,
    {
        Ok(TrackedMut(world.borrow()?))
    }
}

impl<'a, T: 'static + Send + Sync> AllStoragesBorrow<'a> for TrackedMutBorrower<T> {
    fn all_borrow(all_storages: &'a AllStorages) -> Result<Self::View, error::GetStorage>
    where
        Self: Sized,
    {
        Ok(TrackedMut(all_storages.borrow()?))
    }
}

unsafe impl<'a, T: 'static + Send + Sync> BorrowInfo for TrackedMut<'a, T> {
    fn borrow_info(mut info: &mut Vec<info::TypeInfo>) {
        UniqueViewMut::<'a, T>::borrow_info(&mut info);
    }
}

#[derive(PartialEq)]
enum InnerTrackedState {
    New,
    Modified,
    NoChanges,
}

impl<T> TrackedValue<T> {
    pub(crate) fn new(value: T) -> Self {
        TrackedValue(InnerTrackedState::New, value)
    }
    fn reset_tracking(&mut self) {
        self.0 = InnerTrackedState::NoChanges;
    }
    fn is_new_or_modified(&self) -> bool {
        self.0 != InnerTrackedState::NoChanges
    }
}

impl<T> Tracked<'_, T> {
    /// You may only check if Tracked is new or modified for now.
    pub fn is_new_or_modified(&self) -> bool {
        self.0.is_new_or_modified()
    }
}

impl<T: 'static> Deref for Tracked<'_, T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &T {
        &self.0 .1
    }
}

impl<T: 'static> Deref for TrackedMut<'_, T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &T {
        &self.0 .1
    }
}

impl<T: 'static> DerefMut for TrackedMut<'_, T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut T {
        self.0 .0 = InnerTrackedState::Modified;
        &mut self.0 .1
    }
}

impl<T: 'static> AsRef<T> for Tracked<'_, T> {
    fn as_ref(&self) -> &T {
        &**self
    }
}

impl<T: 'static> AsRef<T> for TrackedMut<'_, T> {
    fn as_ref(&self) -> &T {
        &**self
    }
}

impl<T: 'static> AsMut<T> for TrackedMut<'_, T> {
    fn as_mut(&mut self) -> &mut T {
        &mut **self
    }
}

impl<T: 'static + fmt::Display> fmt::Display for Tracked<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<T: 'static + fmt::Debug> fmt::Debug for Tracked<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

/// Add [TrackedUnique] of `T` and reset tracking at the end of every update.
#[derive(Default)]
pub struct TrackedUniquePlugin<T: Clone + Send + Sync + 'static>(T);

impl<T: Clone + Send + Sync + 'static> TrackedUniquePlugin<T> {
    pub fn new(initial_value: T) -> Self {
        TrackedUniquePlugin(initial_value)
    }
}

impl<T: Clone + Send + Sync + 'static> Plugin for TrackedUniquePlugin<T> {
    fn build(&self, app: &mut AppBuilder) {
        app.add_unique(TrackedValue::new(self.0.clone()));
    }
}

pub(crate) fn reset_tracked_unique<T>(mut uvm_tracked_unique_t: UniqueViewMut<TrackedValue<T>>) {
    let span = trace_span!("reset_tracked_unique", tracked = ?type_name::<T>());
    let _span = span.enter();
    uvm_tracked_unique_t.reset_tracking();
}
