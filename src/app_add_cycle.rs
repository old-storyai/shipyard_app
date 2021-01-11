use crate::{App, AppWorkload, AppWorkloadInfo};

macro_rules! fail {
    ($msg:expr $(,)?) => ({ return Err(String::from($msg)) });
    ($fmt:expr, $($arg:tt)+) => ({
        return Err(format!($fmt, $($arg)+))
    });
}

impl App {
    /// Check the ordering of these workloads to check for conflicts.
    ///
    /// Conflicts guarded against:
    ///  * Two different workloads require update_pack for the same storage
    pub fn add_cycle(
        &mut self,
        cycle: Vec<(AppWorkload, AppWorkloadInfo)>,
    ) -> Result<AppWorkload, String> {
        // TODO: Actually perform checks

        // fail!("check_cycle: {:#?}", cycle);

        Ok(AppWorkload(
            cycle
                .iter()
                .flat_map(|(a, _)| a.0.iter().cloned())
                .collect(),
        ))
    }
}
