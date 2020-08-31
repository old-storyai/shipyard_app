# shipyard_app

`shipyard_app` aims to provide a standardized "Plugin" interface on top of the [`shipyard` ECS crate](https://github.com/leudz/shipyard).

This allows for codebases to more easily divide up many systems and workloads without having to declare all systems in one big workload builder in the root of an application.



The initial interface takes a lot of inspiration from [bevy_app]. Thanks @cart!

[bevy_app]: https://github.com/bevyengine/bevy/tree/b925e22949ee1ca990dfc6a678d8e4636cae5271/crates/bevy_app
