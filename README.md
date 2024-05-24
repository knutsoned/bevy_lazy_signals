# bevy_lazy_signals

Primitives and examples for integrating bevy_ecs with signals.

## Rationale

Some of the authors of popular reactive frameworks for JavaScript have been participating in an
effort to create a Signal built-in that will provide a common API to support a range of uses.
For Bevy, such a library could help efforts to integrate UI frameworks, enable networking, support
scripting, scene editing, and file operations.

See also: [Architecture](architecture.md) and [Rationale](rationale.md)

## Dependencies

- [thiserror](https://github.com/dtolnay/thiserror)

## Design Questions

- How to best prevent infinite loops?
- Is the effects system generic enough for consumers to be able to use their own?
- Should a non-lazy version of Immutable be provided?
- (The rest is basically "can this be even lazier?")
- The current system eagerly adds all subscribers up the tree. Is it better to do this in a more
  deferred manner? Seems like it is more trouble to try to track all that than just note which ones
  actually changed at the end and then match the dependencies of effects against that. This assumes
  it is much cheaper to recalculate memos once unnecessarily versus avoiding those recalculations
  with a more complicated processing routine. A more complex Observable might be required.
  Another possibility: update the values directly and use Changed queries somehow.

## General Usage

The LazySignalsPlugin will register a LazySignalsResource which is the main entry point.
Within a system, get the resource as a parameter, then create signals, updating them later.
For basic usage, an application specific resource may track the entities:

TODO: actually test this (see [basic_test](examples/basic_test.rs) for working, tested code)

```
use bevy::prelude::*;
use bevy_lazy_signals::{ api::LazySignal, framework::*, LazySignalsPlugin };

#[derive(Resource, Default)]
struct ConfigResource {
    x_axis: Option<Entity>,
    y_axis: Option<Entity>,
    ...
    action_button: Option<Entity>,
}

fn signals_setup_system(config: Res<ConfigResource>, mut commands: Commands) {
    // note these will not be ready for use until the commands actually run
    config.x_axis = LazySignal.state::<f32>(0.0, commands);
    config.y_axis = LazySignal.state::<f32>(0.0, commands);

    // here we start with an empty Entity (more useful if we already spawned the entity elsewhere)
    config.action_button = commands.spawn_empty().id();

    // then we use the custom command form directly instead
    commands.create_state::<bool>(config.action_button, false);
}

fn signals_update_system(
    config: Res<ConfigResource>,
    mut commands: Commands
) {
    // assume we have somehow read x and y values of the gamepad stick and assigned them to x and y
    LazySignal.send(config.x_axis, x, commands);
    LazySignal.send(config.y_axis, y, commands);

    // signals aren't processed right away, so the signals are still the original value
    let prev_x = LazySignal.read::<f32>(config.x_axis, world);
    let prev_y = LazySignal.read::<f32>(config.y_axis, world);

    // let's force the action button to true to simulate pressing the button but use custom command
    commands.send_signal::<bool>(config.action_button, true);
}
```

TODO: in-depth tutorial (computed memos and effects)
