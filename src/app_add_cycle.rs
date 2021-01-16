use crate::{App, AppWorkload, AppWorkloadInfo, PluginsAssociatedMap};

impl App {
    /// Check the ordering of these workloads to check for conflicts.
    ///
    /// Conflicts guarded against:
    ///  * Two different workloads require update_pack for the same storage
    pub fn add_cycle(
        &mut self,
        cycle: Vec<(AppWorkload, AppWorkloadInfo)>,
    ) -> Result<AppWorkload, String> {
        let mut names_checked = Vec::new();
        let mut cumulative_update_packed =
            PluginsAssociatedMap::new("update packed in previous workloads", &self.type_names);

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
            let mut errs = Vec::new();
            // checks
            for ((up_type, _up_type_name), assoc) in signature.track_update_packed.entries() {
                if !assoc.is_empty() {
                    let (_, plugins_already_claim_update_pack) =
                        cumulative_update_packed.get_plugins(&up_type);
                    if !plugins_already_claim_update_pack.is_empty() {
                        errs.push(format!("Plugin ({:?}) may not claim update pack as it is claimed in an earlier workload's plugins ({:?}).", &name, &plugins_already_claim_update_pack));
                    } else {
                        cumulative_update_packed.associate_all(&up_type, assoc);
                    }
                }
            }
            // if all goes well, add to cumulatives
            if !errs.is_empty() {
                return Err(format!(
                    "Cycle check found errors in workload:\n * {}",
                    errs.join("\n * ")
                ));
            }

            names_checked.push(name);
        }

        Ok(AppWorkload {
            names: names_checked,
        })
    }
}

#[cfg(test)]
mod update_pack_tests {
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
        let error_message = result.expect_err("expected conflict");
        assert!(
            error_message.contains("may not claim update pack"),
            "{}",
            error_message
        );
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
