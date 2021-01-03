use shipyard::*;

/// Simple helper which allows for multiple stages, each with individual [WorkloadBuilder]s.
pub(crate) struct Workloads {
    pub(crate) ordered: Vec<(&'static str, WorkloadBuilder)>,
}

impl Workloads {
    pub(crate) fn new() -> Self {
        Self {
            ordered: Vec::new(),
        }
    }

    pub(crate) fn add_stage(&mut self, stage: &'static str) {
        for (name, _) in self.ordered.iter() {
            if *name == stage {
                return;
            }
        }

        self.ordered.push((stage, WorkloadBuilder::new(stage)));
    }

    // pub(crate) fn add_systems_to_stage<F>(&mut self, stage_name: &'static str, apply_fn: F)
    // where
    //     F: FnOnce(&mut WorkloadBuilder),
    // {
    //     // store so we can take if it's called, and address borrow checker issues that move the apply_fn
    //     let mut apply_fn_opt = Some(apply_fn);
    //     self.ordered = self
    //         .ordered
    //         .drain(..)
    //         .map(|(name, mut workload_builder)| {
    //             if name == stage_name {
    //                 if let Some(apply_fn_first_time) = apply_fn_opt.take() {
    //                     apply_fn_first_time(&mut workload_builder);
    //                 }
    //             }

    //             (name, workload_builder)
    //         })
    //         .collect();

    //     if apply_fn_opt.is_some() {
    //         // apply function not called
    //         panic!("unknown stage '{}'", stage_name)
    //     }
    // }

    pub(crate) fn add_system_to_stage(&mut self, stage_name: &'static str, system: WorkloadSystem) {
        // store so we can take if it's called, and address borrow checker issues that move the apply_fn
        let mut apply_sys_opt = Some(system);
        self.ordered = self
            .ordered
            .drain(..)
            .map(|(name, mut workload_builder)| {
                if name == stage_name {
                    if let Some(apply_sys_first_time) = apply_sys_opt.take() {
                        workload_builder.with_system(Ok(apply_sys_first_time));
                    }
                }

                (name, workload_builder)
            })
            .collect();

        if apply_sys_opt.is_some() {
            // apply function not called
            panic!("unknown stage '{}'", stage_name)
        }
    }
}
