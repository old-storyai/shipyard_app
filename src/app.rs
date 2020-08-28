use crate::app_builder::AppBuilder;
use shipyard::*;
use tracing::*;

#[allow(clippy::needless_doctest_main)]
/// Containers of app logic and data
pub struct App {
    pub world: World,
    pub(crate) update_stages: Vec<&'static str>,
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
    pub fn build() -> AppBuilder {
        AppBuilder::new()
    }

    pub fn update(&self) {
        let span = trace_span!("update");
        let _span = span.enter();
        for update_stage in self.update_stages.iter() {
            let span = trace_span!("update", ?update_stage);
            let _span = span.enter();
            self.world.run_workload(update_stage);
        }
    }

    pub fn run<'s, B, R, S: shipyard::System<'s, (), B, R>>(&'s self, s: S) -> R {
        self.world.try_run(s).unwrap()
    }

    pub fn run_with_data<'s, Data, B, R, S: shipyard::System<'s, (Data,), B, R>>(
        &'s self,
        s: S,
        data: Data,
    ) -> R {
        self.world.try_run_with_data(s, data).unwrap()
    }

    // HMMM...
    // pub fn get_unique<T: Send + Sync + 'static>(mut self) {
    // HMMM...
    // }
}

// /// An event that indicates the app should exit. This will fully exit the app process.
// pub struct AppExit;
