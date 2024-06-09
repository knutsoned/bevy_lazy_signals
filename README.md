# bevy_lazy_signals

Primitives and examples for integrating bevy_ecs with signals.

## [Rationale](rationale.md)

See also: [Architecture](architecture.md)

## Dependencies

- [thiserror](https://github.com/dtolnay/thiserror)

## Design Questions

- How to best prevent infinite loops?
- Should effects have variants for non-exclusive and no world access?

## General Usage

The LazySignalsPlugin will register a LazySignalsResource which is the main entry point.
Within a system, get the resource as a parameter, then create signals, updating them later.
For basic usage, an application specific resource may track the reactive primitive entities.

(see [basic_test](examples/basic_test.rs) for working, tested code)

```
use bevy::prelude::*;
use bevy_lazy_signals::{ api::LazySignals, framework::*, LazySignalsPlugin };

#[derive(Resource, Default)]
struct ConfigResource {
    x_axis: Option<Entity>,
    y_axis: Option<Entity>,
    action_button: Option<Entity>,
    screen_x: Option<Entity>,
    screen_y: Option<Entity>,
    log_effect: Option<Entity>,
}

fn signals_setup_system(config: Res<ConfigResource>, mut commands: Commands) {
    // note these will not be ready for use until the commands actually run
    let x_axis = LazySignals.state::<f32>(0.0, commands);
    config.x_axis = Some(x_axis); // keep as a local to avoid unwrapping for computed and effect sources

    let y_axis = LazySignals.state::<f32>(0.0, commands);
    config.y_axis = Some(y_axis);

    // here we start with an empty Entity (more useful if we already spawned the entity elsewhere)
    config.action_button = commands.spawn_empty().id();

    // then we use the custom command form directly instead
    commands.create_state::<bool>(config.action_button, false);

    // let's define 2 computed values for screen_x and screen_y

    // our x and y axis are mapped to normalized -1.0 to 1.0 OpenGL units and we want 1080p...
    let width = 1920.0;
    let height = 1080.0;

    // the actual pure function to perform the calculations
    let screen_x_fn: Box<dyn Propagator<(f32), f32>> = Box::new(|params, _world| {
        Some(OK(params.0.map_or(0.0, |x| (x + 1.0) * width / 2.0)))
    });

    // and the calculated memo to map the fns to sources and a place to store the result
    let screen_x = LazySignals.computed::<(f32), f32>(
        screen_x_fn,
        vec![x_axis],
        &mut commands
    );

    // or just declare the closure in the API call if it won't be reused
    config.screen_x = Some(screen_x);
    let screen_y = LazySignals.computed::<(f32), f32>(
        Box<dyn Propagator<(f32), f32>> = Box::new(|params, _world| {
            Some(Ok(params.0.map_or(0.0, |y| (y + 1.0) * height / 2.0)))
        }),
        vec![y_axis],
        &mut commands
    );
    config.screen_y = Some(screen_y);

    // at this point screen coordinates will update every time the x or y axis is sent a new signal
    // ...so how do we run an effect?

    // similar in form to making a computed, but we get exclusive world access
    // first the closure
    let effect_fn: Box<dyn Effect<(f32, f32)>> = Box::new(|params, _world| {
        // our inputs are sanitized above so we just unwrap here
        info!("({}, {})", params);
    });

    // then the reactive primitive entity
    config.log_effect = LazySignals.effect::<(f32, f32)>{
        effect_fn,
        vec![], // sources (passed to the params tuple)
        Vec::<Entity>::new(), // triggers (will fire the effect but we don't care about the value)
        &mut commands
    };
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

...configure Bevy per usual, pretty much just init ConfigResource and add the LazySignalsPlugin
```
