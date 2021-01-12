mod app;
mod app_builder;
mod plugin;
mod app_add_cycle;
mod app_changed;

pub use app::*;
pub use app_builder::*;
pub use plugin::*;
pub use shipyard::*;

pub mod prelude {
    pub use crate::{
        app::App,
        app_builder::{AppBuilder, AppWorkload},
        plugin::Plugin,
    };
    pub use shipyard::*;
}

#[cfg(test)]
mod test {
    mod tree;
}
