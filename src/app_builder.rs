#[cfg(feature = "startup-stages")]
use crate::startup_stage;
use crate::{
    app::App,
    // app::{App, AppExit},
    // event::Events,
    plugin::Plugin,
    stage,
};
use shipyard::*;
use tracing::*;
// use bevy_ecs::{FromResources, IntoQuerySystem, Resources, System, World};

/// Configure [App]s using the builder pattern
pub struct AppBuilder {
    pub world: World,
    startup_workloads: Workloads,
    stage_workloads: Workloads,
}

impl Uniques<'_> {
    pub fn add_unique<T: 'static + Send + Sync>(&self, component: T) {
        self.world.add_unique(component)
    }
}

pub struct Uniques<'a> {
    world: &'a World,
}

struct Workloads {
    ordered: Vec<(&'static str, WorkloadBuilder)>,
}

impl Workloads {
    fn new() -> Self {
        Self {
            ordered: Vec::new(),
        }
    }

    fn add_stage(&mut self, stage: &'static str) {
        for (name, _) in self.ordered.iter() {
            if *name == stage {
                return;
            }
        }

        self.ordered.push((stage, WorkloadBuilder::new(stage)));
    }

    fn add_systems_to_stage(&mut self, stage_name: &'static str, apply_fn: impl WorkloadApplyFn) {
        // store so we can take if it's called, and address borrow checker issues that move the apply_fn
        let mut apply_fn_opt = Some(apply_fn);
        self.ordered = self
            .ordered
            .drain(..)
            .map(|(name, mut workload_builder)| {
                if name == stage_name {
                    if let Some(apply_fn_first_time) = apply_fn_opt.take() {
                        apply_fn_first_time(&mut workload_builder);
                        return (name, workload_builder);
                    }
                }

                (name, workload_builder)
            })
            .collect();

        if apply_fn_opt.is_some() {
            // apply function not called
            panic!("unknown stage '{}'", stage_name)
        }
    }
}

pub trait WorkloadApplyFn = Fn(&mut WorkloadBuilder);

impl AppBuilder {
    pub fn new() -> AppBuilder {
        let mut app_builder = AppBuilder::empty();
        app_builder.add_default_stages();
        app_builder
    }

    #[cfg(feature = "startup-stages")]
    fn add_default_stages(&mut self) -> &mut Self {
        self.add_startup_stage(startup_stage::STARTUP)
            .add_startup_stage(startup_stage::POST_STARTUP)
            .add_stage(stage::FIRST)
            .add_stage(stage::EVENT_UPDATE)
            .add_stage(stage::PRE_UPDATE)
            .add_stage(stage::UPDATE)
            .add_stage(stage::POST_UPDATE)
            .add_stage(stage::LAST)
    }

    #[cfg(not(feature = "startup-stages"))]
    fn add_default_stages(&mut self) -> &mut Self {
        self.add_stage(stage::FIRST)
            .add_stage(stage::EVENT_UPDATE)
            .add_stage(stage::PRE_UPDATE)
            .add_stage(stage::UPDATE)
            .add_stage(stage::POST_UPDATE)
            .add_stage(stage::LAST)
    }
}

impl AppBuilder {
    /// In contrast to Bevy, you `finish` to get a function to create the running app, which you can then call the `update` function on.
    ///
    /// So, the general approach to running a Shipyard App is to create a new shipyard [World],
    /// then pass that world into [App::build]. Then, after adding your plugins, you can call this [AppBuilder::start] to get an [App].
    ///
    /// With this App, you can:
    ///  1. Update any Uniques first or use [World::run_with_data] to prime the rest of the systems, then
    ///  2. Call the [App::update()] function, and
    ///  3. Pull any data you need out from the [World], and repeat.
    pub fn finish(self) -> App {
        let AppBuilder {
            world,
            stage_workloads,
            startup_workloads,
        } = self;

        // TODO, how to check if the same world?
        // assert!(&world == setup_world, "same world used for app builder must be used for start");

        startup_workloads
            .ordered
            .into_iter()
            .map(|(name, mut builder)| {
                builder.add_to_world(&world).unwrap();
                world.run_workload(name);
            })
            .count();

        let update_stages: Vec<&'static str> = stage_workloads
            .ordered
            .into_iter()
            .map(|(name, mut builder)| {
                builder.add_to_world(&world).unwrap();
                name
            })
            .collect();

        App {
            world,
            update_stages,
        }
    }

    fn empty() -> AppBuilder {
        let world = shipyard::World::new();
        AppBuilder {
            startup_workloads: Workloads::new(),
            stage_workloads: Workloads::new(),
            world,
        }
    }

    /// Update component `T`'s storage to be update_pack, and add [shipyard::sparse_set::SparseSet::clear_inserted_and_modified] at [stage::LAST].
    pub fn update_pack<T: 'static + Send + Sync>(&mut self) -> &mut Self {
        self.world
            .run(|mut vm_to_pack: ViewMut<T>| vm_to_pack.update_pack());
        self.add_systems_to_stage(
            stage::LAST,
            Box::new(|workload: &mut WorkloadBuilder| {
                workload.with_system(system!(
                    |mut vm_to_clear: ViewMut<T>| vm_to_clear.clear_inserted_and_modified()
                ));
            }),
        )
    }

    /// Update component `T`'s storage to be update_pack, and add [shipyard::sparse_set::SparseSet::clear_inserted_and_modified] at [stage::LAST].
    pub fn add_unique<T>(&mut self, component: T) -> &mut Self
    where
        T: Send + Sync + 'static,
    {
        self.world.add_unique(component);
        self
    }

    // pub fn resources(&self) -> &Resources {
    //     &self.app.resources
    // }

    // pub fn resources_mut(&mut self) -> &mut Resources {
    //     &mut self.app.resources
    // }

    // pub fn run(&mut self) {
    //     let app = std::mem::take(&mut self.world);
    //     app.run();
    // }

    // pub fn set_world(&mut self, world: World) -> &mut Self {
    //     self.world.world = world;
    //     self
    // }

    fn add_stage(&mut self, stage_name: &'static str) -> &mut Self {
        self.stage_workloads.add_stage(stage_name);
        self
    }

    // pub fn add_stage_after(&mut self, target: &'static str, stage_name: &'static str) -> &mut Self {
    //     self.stage_workloads.add_stage_after(target, stage_name);
    //     self
    // }

    // pub fn add_stage_before(
    //     &mut self,
    //     target: &'static str,
    //     stage_name: &'static str,
    // ) -> &mut Self {
    //     self.stage_workloads.add_stage_before(target, stage_name);
    //     self
    // }

    #[cfg(feature = "startup-stages")]
    fn add_startup_stage(&mut self, stage_name: &'static str) -> &mut Self {
        self.startup_workloads.add_stage(stage_name);
        self
    }

    // pub fn add_system(&mut self, system: WorkloadApplyFn) -> &mut Self {
    //     self.add_system_to_stage(stage::UPDATE, system)
    // }

    pub fn add_systems(&mut self, systems: impl WorkloadApplyFn) -> &mut Self {
        self.add_systems_to_stage(stage::UPDATE, systems)
    }

    // pub fn init_system(
    //     &mut self,
    //     build: impl FnMut(&mut Resources) -> Box<dyn System>,
    // ) -> &mut Self {
    //     self.init_system_to_stage(stage::UPDATE, build)
    // }

    // pub fn init_system_to_stage(
    //     &mut self,
    //     stage: &'static str,
    //     mut build: impl FnMut(&mut Resources) -> Box<dyn System>,
    // ) -> &mut Self {
    //     let system = build(&mut self.world.resources);
    //     self.add_system_to_stage(stage, system)
    // }

    // pub fn add_startup_system_to_stage(
    //     &mut self,
    //     stage_name: &'static str,
    //     system: Box<dyn System>,
    // ) -> &mut Self {
    //     self.world
    //         .startup_schedule
    //         .add_system_to_stage(stage_name, system);
    //     self
    // }

    #[cfg(feature = "startup-stages")]
    pub fn add_startup_systems_to_stage(
        &mut self,
        stage_name: &'static str,
        apply_fn: impl WorkloadApplyFn,
    ) -> &mut Self {
        self.startup_workloads
            .add_systems_to_stage(stage_name, apply_fn);
        self
    }

    // pub fn add_startup_system(&mut self, system: Box<dyn System>) -> &mut Self {
    //     self.world
    //         .startup_schedule
    //         .add_system_to_stage(startup_stage::STARTUP, system);
    //     self
    // }

    #[cfg(feature = "startup-stages")]
    pub fn add_startup_systems(&mut self, systems: impl WorkloadApplyFn) -> &mut Self {
        self.add_startup_systems_to_stage(startup_stage::STARTUP, systems)
    }

    // #[cfg(feature = "startup-stages")]
    // pub fn init_startup_system(
    //     &mut self,
    //     build: impl FnMut(&mut Uniques) -> dyn WorkloadApplyFn,
    // ) -> &mut Self {
    //     self.init_startup_systems_to_stage(startup_stage::STARTUP, build)
    // }

    // #[cfg(feature = "startup-stages")]
    // pub fn init_startup_systems_to_stage(
    //     &mut self,
    //     stage: &'static str,
    //     mut build: impl FnMut(&mut Uniques) -> dyn WorkloadApplyFn,
    // ) -> &mut Self {
    //     self.add_startup_systems_to_stage(
    //         stage,
    //         build(&mut Uniques {
    //             world: self.world.clone(),
    //         }),
    //     )
    // }

    pub fn add_systems_to_stage(
        &mut self,
        stage_name: &'static str,
        apply_fn: impl WorkloadApplyFn,
    ) -> &mut Self {
        self.stage_workloads
            .add_systems_to_stage(stage_name, apply_fn);

        self
    }

    // pub fn add_system_to_stage_front(
    //     &mut self,
    //     stage_name: &'static str,
    //     system: Box<dyn System>,
    // ) -> &mut Self {
    //     self.world
    //         .schedule
    //         .add_system_to_stage_front(stage_name, system);
    //     self
    // }

    // pub fn add_systems_to_stage(
    //     &mut self,
    //     stage_name: &'static str,
    //     systems: Vec<Box<dyn System>>,
    // ) -> &mut Self {
    //     for system in systems {
    //         self.stage_workloads.add_system_to_stage(stage_name, system);
    //     }
    //     self
    // }

    // pub fn add_event<T>(&mut self) -> &mut Self
    // where
    //     T: Send + Sync + 'static,
    // {
    //     self.add_resource(Events::<T>::default())
    //         .add_system_to_stage(stage::EVENT_UPDATE, Events::<T>::update_system.system())
    // }

    // pub fn init_resource<R>(&mut self) -> &mut Self
    // where
    //     R: FromResources + Send + Sync + 'static,
    // {
    //     let resource = R::from_resources(&self.world.resources);
    //     self.world.resources.insert(resource);

    //     self
    // }

    // pub fn set_runner(&mut self, run_fn: impl Fn(App) + 'static) -> &mut Self {
    //     self.world.runner = Box::new(run_fn);
    //     self
    // }

    // pub fn load_plugin(&mut self, path: &str) -> &mut Self {
    //     let (_lib, plugin) = dynamically_load_plugin(path);
    //     debug!("loaded plugin: {}", plugin.name());
    //     plugin.build(self);
    //     self
    // }

    pub fn add_plugin<T>(&mut self, plugin: T) -> &mut Self
    where
        T: Plugin,
    {
        debug!("added plugin: {}", plugin.name());
        plugin.build(self);
        self
    }
}
