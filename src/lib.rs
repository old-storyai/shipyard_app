#![feature(trait_alias)]

/// The names of the default App stages
pub mod stage;

/// The names of the default App startup stages
#[cfg(feature = "startup-stages")]
pub mod startup_stage;

mod app;
mod app_builder;
mod event;
mod event_plugin;
mod plugin;
// mod schedule_runner;

pub use app::*;
pub use app_builder::*;
pub use event::*;
pub use event_plugin::EventPlugin;
pub use plugin::*;
// pub use schedule_runner::*;

pub mod prelude {
    pub use crate::{
        app::App,
        app_builder::AppBuilder,
        event::{EventReader, Events},
        plugin::Plugin,
        stage,
    };
    pub use shipyard::*;
}

#[cfg(test)]
mod test {
    mod tree;
}
