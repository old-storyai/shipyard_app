use crate::prelude::*;
use std::marker::PhantomData;

pub struct EventPlugin<T> {
    marker: PhantomData<T>,
}

impl<T> Default for EventPlugin<T> {
    fn default() -> Self {
        EventPlugin {
            marker: PhantomData,
        }
    }
}

impl<T> Plugin for EventPlugin<T>
where
    T: Send + Sync + 'static,
{
    fn build<'a>(&self, app: &mut AppBuilder) {
        app.add_unique(Events::<T>::default()).add_systems_to_stage(
            stage::EVENT_UPDATE,
            |workload| {
                workload.with_system(system!(update_events::<T>));
            },
        );
    }
}

fn update_events<T>(mut uvm_events: UniqueViewMut<Events<T>>)
where
    T: Send + Sync + 'static,
{
    uvm_events.update();
}
