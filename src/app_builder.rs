#[cfg(feature = "startup-stages")]
use crate::startup_stage;
use crate::{app::App, plugin::Plugin, stage};
use shipyard::*;
use std::{
    any::{type_name, TypeId},
    collections::HashMap,
};
use tracing::*;

mod plugin_id;
mod workloads;
use plugin_id::PluginId;
use workloads::Workloads;

/// Configure [App]s using the builder pattern
pub struct AppBuilder {
    pub world: World,
    stage_workloads: Workloads,
    startup_workloads: Workloads,
    /// track the plugins previously added to enable checking that plugin peer dependencies are satisified
    track_added_plugins: HashMap<TypeId, PluginId>,
    /// track the currently being used plugin ([PluginId] is a stack since some plugins add other plugins creating a nest)
    // TODO: Track "Plugin"s for each thing
    track_current_plugin: PluginId,
    /// take a record of type names as we come across them for diagnostics
    track_type_names: HashMap<TypeId, &'static str>,
    /// unique type id to list of plugin type ids that provided a value for it it
    track_uniques: HashMap<TypeId, Vec<PluginId>>,
    /// unique type id to list of (plugin type id, reason string)
    track_unique_dependencies: HashMap<TypeId, Vec<(PluginId, &'static str)>>,
    /// update component storage type id to list of (plugin type id, reason string)
    track_update_packed: HashMap<TypeId, Vec<(PluginId, &'static str)>>,
}

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
    /// The general approach to running a Shipyard App is to create a new shipyard [World],
    /// then pass that world into [App::build]. Then, after adding your plugins, you can call this [AppBuilder::finish] to get an [App].
    ///
    /// With this App, you can:
    ///  1. Update any Uniques first or use [World::run_with_data] to prime the rest of the systems, then
    ///  2. Call the [App::update()] function, and
    ///  3. Pull any data you need out from the [World], and repeat.
    ///
    /// # Panics
    /// May panic if there are unmet unique dependencies or if there is an error adding workloads to shipyard.
    #[track_caller]
    pub fn finish(self) -> App {
        let AppBuilder {
            world,
            stage_workloads,
            startup_workloads,
            track_added_plugins: _,
            track_current_plugin: _,
            track_type_names,
            track_update_packed: _,
            track_uniques,
            mut track_unique_dependencies,
        } = self;

        // trace! out Unique dependencies for diagnostics
        for (unique_type_id, provided_by) in track_uniques {
            let depended_on_by: Vec<(PluginId, &'static str)> = track_unique_dependencies
                .remove(&unique_type_id)
                .unwrap_or_default()
                .into_iter()
                .map(|(dependent_plugin_id, reason)| (dependent_plugin_id, reason))
                .collect();

            let unique_type_name = *track_type_names.get(&unique_type_id).unwrap();
            if provided_by.len() > 1 {
                warn!(name = ?unique_type_name, ?provided_by, ?depended_on_by, "Unique defined by multiple Plugins, only the last registered plugin's unique will be used at startup");
            }

            // good to go
            trace!(name = ?unique_type_name, ?provided_by, ?depended_on_by, "Unique");
        }

        // assert there are no remaining unique dependencies
        let remaining_unique_deps = track_unique_dependencies
            .into_iter()
            .map(|(unique_type_id, dependents)| {
                let unique_type_name = *track_type_names.get(&unique_type_id).unwrap();
                // type name, reason pair
                let depended_on_by: Vec<(PluginId, &'static str)> = dependents
                    .into_iter()
                    .map(|(dependent_plugin_id, reason)| (dependent_plugin_id, reason))
                    .collect();
                format!("- {} required by: {:?}", unique_type_name, depended_on_by)
            })
            .collect::<Vec<String>>();

        if !remaining_unique_deps.is_empty() {
            panic!(
                "Failed to finish app due to unmet unique dependencies:\n{}\n\n{}",
                remaining_unique_deps.join("\n"),
                " * You can add the unique using AppBuilder::add_unique or remove the AppBuilder::add_unique_dependency(s) to resolve this issue."
            );
        }

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
            stage_workloads: Workloads::new(),
            startup_workloads: Workloads::new(),
            track_added_plugins: Default::default(),
            track_current_plugin: Default::default(),
            track_type_names: Default::default(),
            track_uniques: Default::default(),
            track_unique_dependencies: Default::default(),
            track_update_packed: Default::default(),
            world,
        }
    }

    /// Lookup the type id while simultaneously storing the type name to be referenced later
    fn tracked_type_id_of<T: 'static>(&mut self) -> TypeId {
        let type_id = TypeId::of::<T>();
        if !self.track_type_names.contains_key(&type_id) {
            self.track_type_names.insert(type_id, type_name::<T>());
        }

        type_id
    }

    /// Update component `T`'s storage to be update_pack, and add [shipyard::sparse_set::SparseSet::clear_inserted_and_modified] at [stage::LAST].
    pub fn update_pack<T: 'static + Send + Sync>(&mut self, reason: &'static str) -> &mut Self {
        let type_id = self.tracked_type_id_of::<T>();
        match self.track_update_packed.get_mut(&type_id) {
            Some(ref mut list) => {
                list.push((self.track_current_plugin.clone(), reason));
                // no need to pack again
                self
            }
            None => {
                self.track_update_packed
                    .insert(type_id, vec![(self.track_current_plugin.clone(), reason)]);
                self.world
                    .run(|mut vm_to_pack: ViewMut<T>| vm_to_pack.update_pack());
                self.add_systems_to_stage(stage::LAST, |workload| {
                    workload.with_system(system!(
                        |mut vm_to_clear: ViewMut<T>| vm_to_clear.clear_inserted_and_modified()
                    ));
                })
            }
        }
    }

    /// Add a unique component
    pub fn add_unique<T>(&mut self, component: T) -> &mut Self
    where
        T: Send + Sync + 'static,
    {
        self.world.add_unique(component);
        let unique_type_id = self.tracked_type_id_of::<T>();
        self.track_uniques
            .entry(unique_type_id)
            .or_default()
            .push(self.track_current_plugin.clone());
        self
    }

    /// Declare that this builder has a dependency on the following unique.
    ///
    /// If the unique dependency is not satisfied by the time [AppBuilder::finish] is called, then the finish call will panic.
    pub fn depends_on_unique<T>(&mut self, dependency_reason: &'static str) -> &mut Self
    where
        T: Send + Sync + 'static,
    {
        let unique_type_id = self.tracked_type_id_of::<T>();
        self.track_unique_dependencies
            .entry(unique_type_id)
            .or_default()
            .push((self.track_current_plugin.clone(), dependency_reason));
        self
    }

    /// Declare that this builder has a dependency on the following plugin.
    pub fn depends_on_plugin<T>(&mut self, dependency_reason: &'static str) -> &mut Self
    where
        T: Plugin,
    {
        let plugin_type_id = self.tracked_type_id_of::<T>();
        if !self.track_added_plugins.contains_key(&plugin_type_id) {
            panic!(
                "\"{}\" depends on \"{}\": {}",
                self.track_current_plugin,
                type_name::<T>(),
                dependency_reason
            );
        }
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

    pub fn add_systems<F>(&mut self, workload_builder: F) -> &mut Self
    where
        F: FnOnce(&mut WorkloadBuilder),
    {
        self.add_systems_to_stage(stage::UPDATE, workload_builder)
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
    pub fn add_startup_systems_to_stage<F>(
        &mut self,
        stage_name: &'static str,
        workload_builder: F,
    ) -> &mut Self
    where
        F: FnOnce(&mut WorkloadBuilder),
    {
        self.startup_workloads
            .add_systems_to_stage(stage_name, workload_builder);
        self
    }

    // pub fn add_startup_system(&mut self, system: Box<dyn System>) -> &mut Self {
    //     self.world
    //         .startup_schedule
    //         .add_system_to_stage(startup_stage::STARTUP, system);
    //     self
    // }

    #[cfg(feature = "startup-stages")]
    pub fn add_startup_systems(&mut self, workload_builder: impl WorkloadApplyFn) -> &mut Self {
        self.add_startup_systems_to_stage(startup_stage::STARTUP, workload_builder)
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

    pub fn add_systems_to_stage<F>(
        &mut self,
        stage_name: &'static str,
        workload_builder: F,
    ) -> &mut Self
    where
        F: FnOnce(&mut WorkloadBuilder),
    {
        self.stage_workloads
            .add_systems_to_stage(stage_name, workload_builder);

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

    pub fn add_event<T>(&mut self) -> &mut Self
    where
        T: Send + Sync + 'static,
    {
        self.add_plugin(crate::EventPlugin::<T>::default())
    }

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
        let plugin_type_id = self.tracked_type_id_of::<T>();
        if let Some(plugin_id) = self.track_added_plugins.get(&plugin_type_id) {
            panic!(
                "Plugin ({}) cannot add plugin as it's already added as \"{}\"",
                self.track_current_plugin, plugin_id
            );
        }

        if self.track_current_plugin.contains(plugin_type_id) {
            panic!(
                "Plugin ({}) cannot add plugin ({}) as it would cause a cycle",
                self.track_current_plugin,
                self.track_type_names.get(&plugin_type_id).unwrap_or(&""),
            );
        }

        self.track_current_plugin.push::<T>();
        plugin.build(self);
        trace!("added plugin: {}", self.track_current_plugin);
        self.track_added_plugins
            .insert(plugin_type_id, self.track_current_plugin.clone());
        self.track_current_plugin.pop();
        self
    }
}
