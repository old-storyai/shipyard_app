use std::{any::TypeId, borrow::Cow, collections::HashSet};

use crate::{App, AppWorkload, AppWorkloadInfo, PluginAssociated, TypeIdBuckets};

#[derive(Clone)]
pub struct CyclePluginAssociations {
    workload: Cow<'static, str>,
    plugin_id: Option<TypeId>,
    plugins: Vec<PluginAssociated>,
}

impl std::fmt::Debug for CyclePluginAssociations {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(&self.workload)
            .field("plugins", &self.plugins)
            .finish()
    }
}

#[derive(Debug)]
pub enum CycleCheckError {
    UpdatePackResetInMultipleWorkloads {
        update_pack: &'static str,
        conflicts: Vec<CyclePluginAssociations>,
    },
    TrackedUniqueResetInMultipleWorkloads {
        tracked_unique: &'static str,
        conflicts: Vec<CyclePluginAssociations>,
    },
}

impl App {
    /// Check the ordering of these workloads to check for conflicts.
    ///
    /// Conflicts guarded against:
    ///  * Two different workloads require update_pack for the same storage
    pub fn add_cycle(
        &mut self,
        cycle: Vec<(AppWorkload, AppWorkloadInfo)>,
    ) -> Result<AppWorkload, Vec<CycleCheckError>> {
        let mut plugins_added = HashSet::new();
        let mut names_checked = Vec::new();
        let mut cumulative_update_packed = TypeIdBuckets::<CyclePluginAssociations>::new(
            "update packed in workloads",
            &self.type_names,
        );
        let mut cumulative_tracked_uniques = TypeIdBuckets::<CyclePluginAssociations>::new(
            "tracked uniques in workloads",
            &self.type_names,
        );

        'each_workload: for (
            _workloads,
            AppWorkloadInfo {
                name,
                plugin_id,
                signature,
                batch_info: _,
                type_names: _,
            },
        ) in cycle
        {
            names_checked.push(name.clone());

            // can happen if a cycle has the same workload multiple times
            if let Some(ref p) = plugin_id {
                if plugins_added.contains(p) {
                    // so, we don't want to add duplicate associations for them
                    continue 'each_workload;
                } else {
                    plugins_added.insert(p.clone());
                }
            }

            // account for update packs
            for ((up_type, _), assoc) in signature.track_update_packed.entries() {
                if !assoc.is_empty() {
                    cumulative_update_packed.associate(
                        up_type.clone(),
                        CyclePluginAssociations {
                            plugins: assoc,
                            plugin_id: plugin_id.clone(),
                            workload: name.clone(),
                        },
                    );
                }
            }
            // account for tracked uniques
            for ((tracked_type, _), assoc) in signature.track_tracked_uniques.entries() {
                if !assoc.is_empty() {
                    cumulative_tracked_uniques.associate(
                        tracked_type.clone(),
                        CyclePluginAssociations {
                            plugins: assoc,
                            workload: name.clone(),
                            plugin_id: None,
                        },
                    );
                }
            }
        }

        let mut errs = Vec::<CycleCheckError>::new();

        // update pack
        for ((_, update_pack_storage_name), workloads_dependent) in
            cumulative_update_packed.entries()
        {
            if workloads_dependent.len() > 1 {
                errs.push(CycleCheckError::UpdatePackResetInMultipleWorkloads {
                    update_pack: update_pack_storage_name,
                    conflicts: workloads_dependent,
                })
            }
        }
        // tracked unique
        for ((_, tracked_unique_storage_name), workloads_dependent) in
            cumulative_tracked_uniques.entries()
        {
            if workloads_dependent.len() > 1 {
                errs.push(CycleCheckError::TrackedUniqueResetInMultipleWorkloads {
                    tracked_unique: tracked_unique_storage_name,
                    conflicts: workloads_dependent,
                })
            }
        }

        if !errs.is_empty() {
            return Err(errs);
        }

        Ok(AppWorkload {
            names: names_checked,
        })
    }
}

#[cfg(test)]
mod update_pack_tests {
    use std::any::type_name;

    use super::*;

    struct A;
    struct RxA1;
    struct RxA2;
    /// Can be added multiple times
    struct RxADup;
    struct RxTrackA1;
    struct RxTrackA2;
    /// Can be added multiple times
    struct RxTrackADup;
    struct OtherPlugin;

    fn setup_app() -> App {
        let subscriber = tracing_subscriber::FmtSubscriber::builder()
            .with_env_filter(tracing_subscriber::EnvFilter::new(
                std::env::var("RUST_LOG").unwrap_or_else(|_| "warn".to_string()),
            ))
            .finish();
        let _ = ::tracing::subscriber::set_global_default(subscriber);

        App::new()
    }

    #[test]
    fn test_conflicting_update_packs() {
        let mut app = setup_app();

        // Given added workload 1 depends on [A] being update packed
        let rx_a1 = app.add_plugin_workload_with_info(RxA1);
        // And adding workload 2 depends on SAME update packed [A]
        let rx_a2 = app.add_plugin_workload_with_info(RxA2);

        // When declaring in a cycle
        let result = app.add_cycle(vec![rx_a1, rx_a2]);

        // Then observe an error
        let errors = result.expect_err("expected conflict");

        assert_eq!(
            errors.len(),
            1,
            "Expected 1 error, but found: {:#?}",
            errors
        );
        let one_err = errors.first().unwrap();
        if let CycleCheckError::UpdatePackResetInMultipleWorkloads { update_pack, .. } = one_err {
            assert_eq!(*update_pack, type_name::<A>());
        } else {
            panic!(
                "Expected error to be UpdatePackResetInMultipleWorkloads, but found {:#?}",
                one_err
            );
        }
    }

    #[test]
    fn test_duplicates_update_packed_ok() {
        let mut app = setup_app();

        // Given added workload 1 depends on [A] being update packed
        let rx_a_1 = app.add_plugin_workload_with_info(RxADup);
        // And adding the SAME workload 2
        let rx_a_2 = app.add_plugin_workload_with_info(RxADup);

        // When declaring in a cycle
        let result = app.add_cycle(vec![rx_a_1, rx_a_2]);

        // Then observe ok
        let errors = result.expect("expected no conflict");
    }

    #[test]
    fn test_conflicting_tracked() {
        let mut app = setup_app();

        // Given added workload 1 depends on [A] being a tracked unique
        let rx_a1 = app.add_plugin_workload_with_info(RxTrackA1);
        // And adding workload 2 depends on SAME tracked unique [A]
        let rx_a2 = app.add_plugin_workload_with_info(RxTrackA2);

        // When declaring in a cycle
        let result = app.add_cycle(vec![rx_a1, rx_a2]);

        // Then observe an error
        let errors = result.expect_err("expected conflict");

        assert_eq!(
            errors.len(),
            1,
            "Expected 1 error, but found: {:#?}",
            errors
        );
        let one_err = errors.first().unwrap();
        if let CycleCheckError::TrackedUniqueResetInMultipleWorkloads { tracked_unique, .. } =
            one_err
        {
            assert_eq!(*tracked_unique, type_name::<A>());
        } else {
            panic!(
                "Expected error to be TrackedUniqueResetInMultipleWorkloads, but found {:#?}",
                one_err
            );
        }
    }

    #[test]
    fn test_duplicates_tracked_ok() {
        let mut app = setup_app();

        // Given added workload 1 depends on [A] being tracked unique
        let rx_a_1 = app.add_plugin_workload_with_info(RxTrackADup);
        // And adding the SAME workload 2
        let rx_a_2 = app.add_plugin_workload_with_info(RxTrackADup);

        // When declaring in a cycle
        let result = app.add_cycle(vec![rx_a_1, rx_a_2]);

        // Then observe ok
        let errors = result.expect("expected no conflict");
    }

    impl crate::Plugin for RxA1 {
        fn build(&self, app: &mut crate::AppBuilder) {
            app.update_pack::<A>("Rx1");
        }
    }

    impl crate::Plugin for RxA2 {
        fn build(&self, app: &mut crate::AppBuilder) {
            app.update_pack::<A>("Rx2");
        }
    }

    impl crate::Plugin for RxADup {
        fn build(&self, app: &mut crate::AppBuilder) {
            app.update_pack::<A>("RxDup");
        }
        fn can_add_multiple_times(&self) -> bool {
            true
        }
    }

    impl crate::Plugin for OtherPlugin {
        fn build(&self, app: &mut crate::AppBuilder) {}
    }

    impl crate::Plugin for RxTrackA1 {
        fn build(&self, app: &mut crate::AppBuilder) {
            app.tracks::<A>("RxTrack1");
        }
    }

    impl crate::Plugin for RxTrackA2 {
        fn build(&self, app: &mut crate::AppBuilder) {
            app.tracks::<A>("RxTrack2");
        }
    }
    impl crate::Plugin for RxTrackADup {
        fn build(&self, app: &mut crate::AppBuilder) {
            app.tracks::<A>("RxTrackDup");
        }
        fn can_add_multiple_times(&self) -> bool {
            true
        }
    }
}
