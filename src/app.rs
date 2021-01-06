use std::any::type_name;

use crate::{app_builder::AppBuilder, AppWorkload, Plugin};
use shipyard::*;
use tracing::trace_span;

#[allow(clippy::needless_doctest_main)]
/// Containers of app logic and data
pub struct App {
    pub world: World,
    // pub(crate) update_stage: &'static str,
    // pub(crate) startup_stages: Vec<&'static str>,
    // pub resources: Resources,
    // pub runner: Box<dyn Fn(App)>,
    // pub schedule: Schedule,
    // pub executor: ParallelExecutor,
    // pub startup_schedule: Schedule,
    // pub startup_executor: ParallelExecutor,
}

// fn run_once(mut app: App) {
//     app.update();
// }

impl App {
    #[allow(clippy::new_without_default)]
    pub fn new() -> App {
        App::new_with_world(World::new())
    }
    pub fn new_with_world(world: World) -> App {
        App { world }
    }

    #[track_caller]
    pub fn add_plugin_workload<P>(&self, plugin: P) -> AppWorkload
    where
        P: Plugin + 'static,
    {
        let span = trace_span!("add_plugin_workload", plugin = ?type_name::<P>());
        let _span = span.enter();
        let mut builder = AppBuilder::new(&self);
        plugin.build(&mut builder);
        let workload_name = type_name::<P>();
        builder.finish_with_info_named(workload_name.into()).0
    }

    // pub fn update(&self) {
    //     let span = trace_span!("update");
    //     let _span = span.enter();
    //     self.world.run_workload(self.update_stage);
    // }

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

    // HMMM...
    // pub fn get_unique<T: Send + Sync + 'static>(mut self) {
    // HMMM...
    // }
}

// /// An event that indicates the app should exit. This will fully exit the app process.
// pub struct AppExit;
