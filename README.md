# LazySignals for Bevy

#### _An ad hoc, informally-specified, bug-ridden, kinda fast implementation of 1/3 of MIT-Scheme._

---

Primitives and examples for implementing a lazy kind of reactive signals for Bevy.

## [Rationale](rationale.md)

See also: [Architecture](architecture.md)

## Dependencies

- [thiserror](https://github.com/dtolnay/thiserror)

## Design Questions

- Can this work with futures_lite to create a futures-signals-like API?
- During initialization, should computed and effect contexts actually evaluate?
- How to best prevent infinite loops?
- Can the use of get vs unwrap be more consistent?
- Should Tasks be able to renember they were retriggered while still running and then run again immediately after finishing?
- Should there be an option to run a Bevy system as an effect?

## TODO

### Missing

- [ ] Testing
- [ ] Error handling and general resiliency
- [ ] API documentation

### Enhancements

- [ ] I need someone to just review every line because I am a total n00b
- [ ] More examples, including some integrating LazySignals with popular Bevy projects
      such as bevy-lunex, bevy_dioxus, bevy_proto, bevy_reactor, haalka, polako, etc.

### Stuff I'm Actually Going To Do

- [x] Define bundles for the signals primitives
- [x] Support bevy_reflect types out of the box
- [x] Add async task management for effects
- [x] Prevent retrigger if task still running from last time
- [x] Process tasks to run their commands when they are complete
- [ ] Add React-like factory to API (return getter/setter tuples for signals)
- [ ] Prevent infinite loops
- [ ] See how well this plays with aery, bevy_mod_picking, bevy_mod_scripting, and sickle
- [ ] Do the [Ten Challenges](https://github.com/bevyengine/bevy/discussions/11100)
- [ ] Write a bunch of Fennel code to see how well it works to script the computeds and effects

## General Usage

The LazySignalsPlugin will register a LazySignalsResource which is the main reactive context.
Within a system, get the resource from world scope, then create signals, updating them later.
For basic usage, an application specific resource may track the reactive primitive entities.

(see [basic_test](examples/basic_test.rs) for working, tested code)

```
use bevy::prelude::*;
use bevy_lazy_signals::{
    api::LazySignals,
    commands::LazySignalsCommandsExt,
    framework::*,
    LazySignalsPlugin
};

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
    let screen_x_fn: Box<dyn Propagator<(f32), f32>> = Box::new(|args, _world| {
        Some(OK(args.0.map_or(0.0, |x| (x + 1.0) * width / 2.0)))
    });

    // and the calculated memo to map the fns to sources and a place to store the result
    let screen_x = LazySignals.computed::<(f32), f32>(
        screen_x_fn,
        vec![x_axis],
        &mut commands
    );
    config.screen_x = Some(screen_x);

    // or just declare the closure in the API call if it won't be reused
    let screen_y = LazySignals.computed::<(f32), f32>(
        Box<dyn Propagator<(f32), f32>> = Box::new(|args, _world| {
            Some(Ok(args.0.map_or(0.0, |y| (y + 1.0) * height / 2.0)))
        }),
        vec![y_axis],
        &mut commands
    );
    config.screen_y = Some(screen_y);

    // at this point screen coordinates will update every time the x or y axis is sent a new signal
    // ...so how do we run an effect?

    // similar in form to making a computed, but we get exclusive world access
    // first the closure
    let effect_fn: Box<dyn Effect<(f32, f32)>> = Box::new(|args, _world| {
        let x = args.0.map_or("???", |x| format!("{:.1}", x))
        let y = args.0.map_or("???", |y| format!("{:.1}", y))
        info!(format!("({}, {})"), x, y)
    });

    // then the reactive primitive entity, which logs the screen position every time the HID moves
    config.log_effect = LazySignals.effect::<(f32, f32)>{
        effect_fn,
        vec![screen_x, screen_y], // sources (passed to the args tuple)
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
    commands.
    LazySignal.send(config.y_axis, y, commands);

    // signals aren't processed right away, so the signals are still the original value
    let prev_x = LazySignal.read::<f32>(config.x_axis, world);
    let prev_y = LazySignal.read::<f32>(config.y_axis, world);

    // let's force the action button to true to simulate pressing the button but use custom command
    commands.send_signal::<bool>(config.action_button, true);
}

...configure Bevy per usual, pretty much just init ConfigResource and add the LazySignalsPlugin
```

# License

All code in this repository is dual-licensed under either:

- MIT License (LICENSE-MIT or http://opensource.org/licenses/MIT)
- Apache License, Version 2.0 (LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0)

at your option. This means you can select the license you prefer.

## Your contributions

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
