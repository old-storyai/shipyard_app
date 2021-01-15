use std::any::type_name;

use crate::{app_builder::AppBuilder, type_names::TypeNames, AppWorkload, AppWorkloadInfo, Plugin};
use shipyard::*;
use tracing::trace_span;

#[allow(clippy::needless_doctest_main)]
/// Containers of app logic and data
pub struct App {
    pub world: World,
    pub(crate) type_names: TypeNames,
}

impl App {
    #[allow(clippy::new_without_default)]
    pub fn new() -> App {
        App::new_with_world(World::new())
    }
    pub fn new_with_world(world: World) -> App {
        App {
            world,
            type_names: TypeNames::default(),
        }
    }

    #[track_caller]
    pub fn add_plugin_workload<P>(&self, plugin: P) -> AppWorkload
    where
        P: Plugin + 'static,
    {
        self.add_plugin_workload_with_info(plugin).0
    }

    #[track_caller]
    pub fn add_plugin_workload_with_info<P>(&self, plugin: P) -> (AppWorkload, AppWorkloadInfo)
    where
        P: Plugin + 'static,
    {
        let span = trace_span!("add_plugin_workload_with_info", plugin = ?type_name::<P>());
        let _span = span.enter();
        let mut builder = AppBuilder::new(&self);
        plugin.build(&mut builder);
        let workload_name = type_name::<P>();
        builder.finish_with_info_named(workload_name.into())
    }

    /// Runs default workload
    #[track_caller]
    pub fn update(&self) {
        let span = trace_span!("update");
        let _span = span.enter();
        self.world.run_default().unwrap();
    }

    #[track_caller]
    pub fn run<'s, B, R, S: shipyard::System<'s, (), B, R>>(&'s self, s: S) -> R {
        self.world.run(s).unwrap()
    }

    #[track_caller]
    pub fn run_with_data<'s, Data, B, R, S: shipyard::System<'s, (Data,), B, R>>(
        &'s self,
        s: S,
        data: Data,
    ) -> R {
        self.world.run_with_data(s, data).unwrap()
    }
}
