use std::borrow::Cow;

use crate::{App, AppWorkload, AppWorkloadInfo, PluginAssociated, TypeIdBuckets};

#[derive(Clone, Debug)]
pub struct CyclePluginAssociations {
    workload: Cow<'static, str>,
    plugins: Vec<PluginAssociated>,
}

#[derive(Debug)]
pub enum CycleCheckError {
    UpdatePackDeclaredInMultipleWorkloads {
        update_pack: &'static str,
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
        let mut names_checked = Vec::new();
        let mut cumulative_update_packed = TypeIdBuckets::<CyclePluginAssociations>::new(
            "update packed in previous workloads",
            &self.type_names,
        );

        for (
            _workloads,
            AppWorkloadInfo {
                name,
                signature,
                batch_info: _,
                type_names: _,
            },
        ) in cycle
        {
            // checks
            for ((up_type, _), assoc) in signature.track_update_packed.entries() {
                if !assoc.is_empty() {
                    cumulative_update_packed.associate(
                        up_type.clone(),
                        CyclePluginAssociations {
                            plugins: assoc,
                            workload: name.clone(),
                        },
                    );
                }
            }

            names_checked.push(name);
        }

        let mut errs = Vec::<CycleCheckError>::new();

        // if all goes well, add to cumulatives
        for ((_, update_pack_storage_name), workloads_dependent) in
            cumulative_update_packed.entries()
        {
            if workloads_dependent.len() > 1 {
                errs.push(CycleCheckError::UpdatePackDeclaredInMultipleWorkloads {
                    update_pack: update_pack_storage_name,
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
    // struct B;
    struct RxA1;
    struct RxA2;

    fn setup_app() -> App {
        let subscriber = tracing_subscriber::FmtSubscriber::builder()
            .with_env_filter(tracing_subscriber::EnvFilter::new(
                std::env::var("RUST_LOG").unwrap_or_else(|_| "warn".to_string()),
            ))
            .finish();
        ::tracing::subscriber::set_global_default(subscriber).unwrap();

        App::new()
    }

    #[test]
    fn test_conflicting() {
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
        if let CycleCheckError::UpdatePackDeclaredInMultipleWorkloads { update_pack, .. } = one_err
        {
            assert_eq!(*update_pack, type_name::<A>());
        } else {
            panic!(
                "Expected error to be UpdatePackDeclaredInMultipleWorkloads, but found {:#?}",
                one_err
            );
        }
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
}
