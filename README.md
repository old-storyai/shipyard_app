# shipyard_app

`shipyard_app` aims to provide a standardized "Plugin" interface on top of the [`shipyard` ECS crate](https://github.com/leudz/shipyard).

This allows for codebases to more easily divide up many systems and workloads without having to declare all systems in one big workload builder in the root of an application.


Example [from test/tree.rs](https://github.com/storyscript/shipyard_app/blob/master/src/test/tree.rs)
```rust
use shipyard_app::{AppBuilder, EventPlugin, Plugin, stage};
use shipyard::{system, WorkloadBuilder};
...

/// Registers
#[derive(Default)]
pub struct TreePlugin;

impl Plugin for TreePlugin {
    fn build<'a>(&self, app: &mut AppBuilder) {
        app.add_plugin(EventPlugin::<reordering::MoveCmd>::default())
            .update_pack::<ChildOf>() // enable change tracking in shipyard for the ChildOf component
            .add_systems_to_stage(stage::POST_UPDATE, |workload: &mut WorkloadBuilder| {
                workload
                    .with_system(system!(reordering::tree_reordering))
                    .with_system(system!(indexing::tree_indexing));
            });
    }
}
```



The initial interface takes a lot of inspiration from [bevy_app]. Thanks @cart!

[bevy_app]: https://github.com/bevyengine/bevy/tree/b925e22949ee1ca990dfc6a678d8e4636cae5271/crates/bevy_app
