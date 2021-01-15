use crate::{app::App, plugin::Plugin, type_names::TypeNames};
use shipyard::*;
use std::{
    any::{type_name, TypeId},
    borrow::Cow,
    collections::hash_map::Entry,
    collections::HashMap,
};
use tracing::*;

mod plugin_id;
use plugin_id::PluginId;

/// Name of app stage responsible for doing most app logic. Systems should be registered here by default.
pub const DEFAULT_STAGE: &str = "default";

struct PluginAssociated {
    plugin: PluginId,
    reason: &'static str,
}

struct PluginsAssociatedMap {
    name: &'static str,
    track_type_names: TypeNames,
    type_plugins_lookup: HashMap<TypeId, Vec<PluginAssociated>>,
}

pub enum AssociateResult {
    First,
    Nth(usize),
}

impl AssociateResult {
    fn is_first(&self) -> bool {
        match self {
            AssociateResult::First => true,
            AssociateResult::Nth(_) => false,
        }
    }
}

impl PluginsAssociatedMap {
    fn new(name: &'static str, track_type_names: &TypeNames) -> Self {
        PluginsAssociatedMap {
            name,
            track_type_names: track_type_names.clone(),
            type_plugins_lookup: Default::default(),
        }
    }

    /// Return new number of plugins associated
    fn associate<T: 'static>(
        &mut self,
        plugin: &PluginId,
        reason: &'static str,
    ) -> AssociateResult {
        let type_id = self.track_type_names.tracked_type_id_of::<T>();

        let assoc = PluginAssociated {
            plugin: plugin.clone(),
            reason,
        };

        match self.type_plugins_lookup.entry(type_id) {
            Entry::Occupied(mut list) => {
                // no need to pack again
                list.get_mut().push(assoc);
                trace!(?plugin, with = ?type_name::<T>(), ?reason, result = "existed", "Associated {}", self.name);
                AssociateResult::Nth(list.get().len())
            }
            Entry::Vacant(list) => {
                list.insert(vec![assoc]);
                trace!(?plugin, with = ?type_name::<T>(), ?reason, result = "added", "Associated {}", self.name);
                AssociateResult::First
            }
        }
    }
}

/// Configure [App]s using the builder pattern
pub struct AppBuilder<'a> {
    pub app: &'a App,
    resets: Vec<WorkloadSystem>,
    systems: Vec<WorkloadSystem>,
    /// track the plugins previously added to enable checking that plugin peer dependencies are satisified
    track_added_plugins: HashMap<TypeId, PluginId>,
    /// track the currently being used plugin ([PluginId] is a stack since some plugins add other plugins creating a nest)
    // TODO: Track "Plugin"s for each thing
    track_current_plugin: PluginId,
    /// take a record of type names as we come across them for diagnostics
    track_type_names: TypeNames,
    /// track the plugins directly required by other plugins
    track_plugin_dependencies: PluginsAssociatedMap,
    /// unique type id to list of plugin type ids that provided a value for it it
    track_uniques_provided: PluginsAssociatedMap,
    /// unique type id to list of (plugin type id, reason string)
    track_unique_dependencies: PluginsAssociatedMap,
    /// update component storage type id to list of (plugin type id, reason string)
    track_update_packed: PluginsAssociatedMap,
}

impl<'a> AppBuilder<'a> {
    pub fn new(app: &App) -> AppBuilder<'_> {
        AppBuilder::empty(app)
    }
}

#[derive(Clone, Debug)]
pub struct CycleWorkload {
    pub(crate) names: Vec<std::borrow::Cow<'static, str>>,
}

#[derive(Clone, Debug)]
pub struct SingleWorkload {
    pub(crate) name: std::borrow::Cow<'static, str>,
    // ... requirements and such
}

#[derive(Clone, Debug)]
pub enum AppWorkload {
    Cycle(CycleWorkload),
    Single(SingleWorkload),
}

impl AppWorkload {
    pub(crate) fn names(&self) -> Vec<std::borrow::Cow<'static, str>> {
        match self {
            AppWorkload::Cycle(c) => c.names.clone(),
            AppWorkload::Single(n) => vec![n.name.clone()],
        }
    }
}

#[derive(Clone, Debug)]
pub struct AppWorkloadInfo {
    pub batch_info: Vec<info::BatchInfo>,
    pub name: Cow<'static, str>,
}

impl AppWorkload {
    #[track_caller]
    #[instrument(skip(app))]
    pub fn run(&self, app: &App) {
        match self {
            AppWorkload::Cycle(CycleWorkload { names }) => {
                for workload_name in names.iter() {
                    let span = trace_span!("AppWorkload::run", ?workload_name);
                    let _span = span.enter();
                    app.world.run_workload(&workload_name).unwrap();
                }
            }
            AppWorkload::Single(SingleWorkload { name }) => {
                let span = trace_span!("AppWorkload::run", workload_name = ?name);
                let _span = span.enter();
                app.world.run_workload(&name).unwrap();
            }
        }
    }
}

impl<'a> AppBuilder<'a> {
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
    pub fn finish(self) -> AppWorkload {
        self.finish_with_info().0
    }

    /// Finish [App] and report back each of the update stages with their [AppWorkloadInfo].
    #[track_caller]
    pub fn finish_with_info(self) -> (AppWorkload, AppWorkloadInfo) {
        self.finish_with_info_named("update".into())
    }

    /// Finish [App] and report back each of the update stages with their [AppWorkloadInfo].
    #[track_caller]
    #[instrument(skip(self))]
    pub(crate) fn finish_with_info_named(
        self,
        update_stage: std::borrow::Cow<'static, str>,
    ) -> (AppWorkload, AppWorkloadInfo) {
        let AppBuilder {
            app,
            resets,
            systems,
            track_added_plugins: _,
            track_current_plugin: _,
            track_type_names,
            track_update_packed: _,
            track_uniques_provided: track_uniques,
            mut track_unique_dependencies,
            track_plugin_dependencies: _,
        } = self;

        let mut update_workload = systems.into_iter().fold(
            WorkloadBuilder::new(update_stage.clone()),
            |mut acc: WorkloadBuilder, system: WorkloadSystem| {
                acc.with_system(system);
                acc
            },
        );

        for reset_system in resets {
            update_workload.with_system(reset_system);
        }

        let info = update_workload.add_to_world_with_info(&app.world).unwrap();
        (
            AppWorkload::Cycle(CycleWorkload {
                names: vec![update_stage],
            }),
            AppWorkloadInfo {
                batch_info: info.batch_info,
                name: info.name,
            },
        )
    }

    /// Lookup the type id while simultaneously storing the type name to be referenced later
    fn tracked_type_id_of<T: 'static>(&mut self) -> TypeId {
        self.track_type_names.tracked_type_id_of::<T>()
    }

    /// Update component `T`'s storage to be update_pack, and add [shipyard::sparse_set::SparseSet::clear_all_inserted_and_modified] as the last system.
    #[track_caller]
    pub fn update_pack<T: 'static + Send + Sync>(&mut self, reason: &'static str) -> &mut Self {
        if self
            .track_update_packed
            .associate::<T>(&self.track_current_plugin, reason)
            .is_first()
        {
            self.app.world.borrow::<ViewMut<T>>().unwrap().update_pack();
            self.resets.push(system!(reset_update_pack::<T>));
        }

        self
    }

    /// Add a unique component
    #[track_caller]
    pub fn add_unique<T>(&mut self, component: T) -> &mut Self
    where
        T: Send + Sync + 'static,
    {
        if self
            .track_uniques_provided
            .associate::<T>(&self.track_current_plugin, "<not provided>")
            .is_first()
        {
            self.app.world.add_unique(component).unwrap();
        } else {
            warn!(
                "Unique({}) already provided by another Plugin",
                type_name::<T>()
            )
        }

        self
    }

    /// Declare that this builder has a dependency on the following unique.
    ///
    /// If the unique dependency is not satisfied by the time [AppBuilder::finish] is called, then the finish call will panic.
    #[track_caller]
    pub fn depends_on_unique<T>(&mut self, dependency_reason: &'static str) -> &mut Self
    where
        T: Send + Sync + 'static,
    {
        self.track_unique_dependencies
            .associate::<T>(&self.track_current_plugin, dependency_reason);

        self
    }

    /// Declare that this builder has a dependency on the following plugin.
    #[track_caller]
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

    fn empty(app: &App) -> AppBuilder<'_> {
        AppBuilder {
            app,
            resets: Vec::new(),
            systems: Vec::new(),
            track_added_plugins: Default::default(),
            track_current_plugin: Default::default(),
            track_type_names: Default::default(),
            track_plugin_dependencies: PluginsAssociatedMap::new(
                "Plugin depends on Plugin",
                &app.type_names,
            ),
            track_uniques_provided: PluginsAssociatedMap::new(
                "Plugin provides Unique",
                &app.type_names,
            ),
            track_unique_dependencies: PluginsAssociatedMap::new(
                "Plugin depends on Unique",
                &app.type_names,
            ),
            track_update_packed: PluginsAssociatedMap::new(
                "Plugin requires update_pack",
                &app.type_names,
            ),
        }
    }

    #[track_caller]
    pub fn add_system(&mut self, system: WorkloadSystem) -> &mut Self {
        self.systems.push(system);

        self
    }

    /// Ensure that this system is among the absolute last systems
    #[track_caller]
    pub fn add_reset_system(&mut self, system: WorkloadSystem, reason: &str) -> &mut Self {
        trace!(plugin = ?self.track_current_plugin, ?reason, "add_reset_system");
        self.resets.push(system);

        self
    }

    #[track_caller]
    pub fn add_plugin<T>(&mut self, plugin: T) -> &mut Self
    where
        T: Plugin,
    {
        let plugin_type_id = self.tracked_type_id_of::<T>();
        let span = trace_span!("add_plugin", plugin = ?self.track_current_plugin, adding = ?type_name::<T>());
        let _span = span.enter();
        if let Some(plugin_id) = self.track_added_plugins.get(&plugin_type_id) {
            if !plugin.can_add_multiple_times() {
                panic!(
                    "Plugin ({}) cannot add plugin as it's already added as \"{}\". (Implement `Plugin::can_add_multiple_times` to override)",
                    self.track_current_plugin, plugin_id
                );
            }
        }

        if self.track_current_plugin.contains(plugin_type_id) {
            panic!(
                "Plugin ({}) cannot add plugin ({}) as it would cause a cycle",
                self.track_current_plugin,
                self.track_type_names
                    .lookup_name(&plugin_type_id)
                    .unwrap_or(""),
            );
        }

        self.track_current_plugin.push::<T>();
        trace_span!("build", plugin = ?self.track_current_plugin).in_scope(|| {
            plugin.build(self);
        });
        self.track_added_plugins
            .insert(plugin_type_id, self.track_current_plugin.clone());
        self.track_current_plugin.pop();
        self
    }
}

fn reset_update_pack<T>(mut vm_to_clear: ViewMut<T>) {
    trace_span!("reset_update_pack", storage_name = type_name::<T>()).in_scope(|| {
        vm_to_clear.clear_all_inserted_and_modified();
        vm_to_clear.take_removed_and_deleted();
    });
}
