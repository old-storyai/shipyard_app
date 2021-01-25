mod app;
mod app_add_cycle;
mod app_builder;
mod plugin;
mod tracked_unique;
mod type_names;

pub use app::*;
pub use app_builder::*;
pub use plugin::*;
pub use shipyard::*;
pub use tracked_unique::*;

pub use app_add_cycle::CycleSummary;

pub mod prelude {
    pub use crate::{
        app::App,
        app_builder::{AppBuilder, AppWorkload},
        plugin::Plugin,
        tracked_unique::{Tracked, TrackedMut},
    };
    pub use shipyard::*;
}

#[cfg(test)]
mod test {
    mod tree;
}
