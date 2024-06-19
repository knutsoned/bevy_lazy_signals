# LazySignals for Bevy

#### _An ad hoc, informally-specified, bug-ridden, kinda fast implementation of 1/3 of MIT-Scheme._

---

Primitives and examples for implementing a lazy kind of reactive signals for [Bevy](https://github.com/bevyengine/bevy).

_WARNING:_ This is under active development and comes with even fewer guarantees for fitness to
purpose than the licenses provide.

## Credits

The initial structure of this project is based on [bevy_rx](https://github.com/aevyrie/bevy_rx).

## Architecture

This library is basically Haskell monads under the hood with a TC39 Signals developer API inspired
by ECMAScript so-called reactive libraries. It is also influenced by the MIT Propagator model.
Well, at least a [YouTube video](https://www.youtube.com/watch?v=HB5TrK7A4pI) that mentions it.

See also in-depth [Architecture](architecture.md) and [Rationale](rationale.md)

## Dependencies

- [thiserror](https://github.com/dtolnay/thiserror)

## Design Questions

- What's a good way to handle errors?
- Can this work with futures_lite to create a futures-signals-like API?
- During initialization, should computed and effect contexts actually evaluate?
- How to best prevent or detect infinite loops?
- Can the use of get vs unwrap be more consistent?
- ✔️ Should Tasks be able to renember they were retriggered while still running and
  then run again immediately after finishing? (I think they currently do)
- ✔️ Should there be an option to run a Bevy system as an effect?
- Should there be a commands-only version of effects?
- Do we need a useRef equivalent to support state that is not passed around by value?
- Same question about useCallback
- ❌ Can change detection replace some of the components we currently add manually?
- Can a Computed and an Effect live on the same entity? (Technically yes, but why?)
- Do we want an API to trigger an Effect directly?
- Should there be a way to write closures that take the result struct and not Option?
- How to send a DynamicStruct as a signal? Doesn't work now due to FromReflect bound.
- Lots of reactive libraries distinguish Actions from Effects. Should AsyncTask be renamed to
  Action?

## TODO

### Missing

- [ ] Testing
- [ ] Error handling and general resiliency

### Enhancements

- [ ] See if there is a way to register effect systems during init and retain SystemId
- [ ] More API documentation
- [ ] I need someone to just review every line because I am a total n00b
- [ ] More examples, including basic game stuff (gold and health seem popular)
- [ ] More examples, including some integrating LazySignals with popular Bevy projects
      such as bevy-lunex, bevy_dioxus, bevy_editor_pls, bevy_reactor, haalka,
      kayak_ui, polako, quill, space_editor, etc.

### Stuff I'm Actually Going To Do

- [x] Define bundles for the signals primitives
- [x] Support bevy_reflect types out of the box
- [x] Add async task management for effects
- [x] Prevent retrigger if task still running from last time
- [x] Process tasks to run their commands when they are complete
- [x] Make sure Triggered gets removed from Computeds during processing
- [x] Remove Clone from LazySignalsData trait bounds
- [x] Implement effect systems
- [ ] Add getter/setter tuples factory to API
- [ ] Add Source fields for sources Vecs
- [ ] Integrate with bevy_mod_picking
- [ ] Make a demo of a fully wired sickle entity inspector with schematics
- [ ] Make sure we can convert the result struct into a regular Option<Result<>>
- [ ] Find a better way to manage the Effect systems (at init time)
- [ ] See if there is a way to schedule a system using an Action's CommandQueue
- [ ] Provide integration with Bevy observers
- [ ] Support undo/redo
- [ ] Integrate with bevy-inspector-egui
- [ ] Do the [Ten Challenges](https://github.com/bevyengine/bevy/discussions/11100)
- [ ] Support streams if the developer expects the same signal to be sent multiple times/tick
- [ ] See how well the demo plays with bevy_mod_scripting
- [ ] Write a bunch of Fennel code to see how well it works to script the computeds and effects
- [ ] Make a visual signals editor plugin
- [ ] See how well the demo plays with aery
- [ ] Prevent or at least detect infinite loops

## General Usage

The LazySignalsPlugin will register the core types and systems.

Create signals, computeds, effects, and tasks with the API during application init. Read and send
signals and read memoized computeds in update systems. Trigger actions and effects when source or
trigger signals are sent or source computeds change value.

For basic usage, an application specific resource may track the reactive primitive entities.

(see [basic_test](examples/basic_test.rs) for working, tested code)

```rust
use bevy::prelude::*;
use bevy_lazy_signals::{
    api::LazySignals,
    commands::LazySignalsCommandsExt,
    framework::*,
    LazySignalsPlugin
};

#[derive(Resource)]
struct ConfigResource {
    x_axis: Entity,
    y_axis: Entity,
    action_button: Entity,
    screen_x: Entity,
    screen_y: Entity,
    log_effect: Entity,
    action: Entity,
}

struct MyActionButtonCommand(Entity);

impl Command for MyActionButtonCommand {
    fn apply(self, world: &mut World) {
        info!("Pushing the button");
        LazySignals.send::<bool>(self.0, true, world.commands());
        world.flush_commands();
    }
}

fn signals_setup_system(mut commands: Commands) {
    // note these will not be ready for use until the commands actually run
    let x_axis = LazySignals.state::<f32>(0.0, commands);

    let y_axis = LazySignals.state::<f32>(0.0, commands);

    // here we start with a new Entity (more useful if we already spawned it elsewhere)
    let action_button = commands.spawn_empty().id();

    // then we use the custom command form directly instead
    commands.create_state::<bool>(action_button, false);

    // let's define 2 computed values for screen_x and screen_y

    // say x and y are mapped to normalized -1.0 to 1.0 OpenGL units and we want 1080p...
    let width = 1920.0;
    let height = 1080.0;

    // the actual pure function to perform the calculations
    let screen_x_fn = |args: (f32)| {
        LazySignals::result(args.0.map_or(0.0, |x| (x + 1.0) * width / 2.0))
    };

    // and the calculated memo to map the fns to sources and a place to store the result
    let screen_x = LazySignals.computed::<(f32), f32>(
        screen_x_fn,
        vec![x_axis],
        &mut commands
    );

    // or just declare the closure in the API call if it won't be reused
    let screen_y = LazySignals.computed::<(f32), f32>(
        // because we pass (f32) as the first type param, the compiler knows type of args here
        |args| {
            LazySignals::result(args.0.map_or(0.0, |y| (y + 1.0) * height / 2.0))
        },
        vec![y_axis],
        &mut commands
    );

    // at this point screen coords will update every time the x or y axis is sent a new signal
    // ...so how do we run an effect?

    // similar in form to making a computed, but we get exclusive world access
    // first the closure (that is an &mut World, if needed)
    let effect_fn = |args: (f32, f32), _world| {
        let x = args.0.map_or("???", |x| format!("{:.1}", x))
        let y = args.1.map_or("???", |y| format!("{:.1}", y))
        info!(format!("({}, {})"), x, y)
    };

    // then the reactive primitive entity, which logs screen position every time the HID moves
    let log_effect = LazySignals.effect::<(f32, f32)>{
        effect_fn,
        vec![screen_x, screen_y], // sources (passed to the args tuple)
        Vec::<Entity>::new(), // triggers (will fire an effect but don't care about the value)
        &mut commands
    };

    // unlike a brief Effect which gets exclusive world access, an Action is an async task
    // but only returns a CommandQueue, to run when the system that checks Bevy tasks notices
    // it has completed
    let action_fn = |args: (f32, f32)| {
        let mut command_queue = CommandQueue::default();

        // as long as the task is still running, it will not spawn another instance
        do_something_that_takes_a_long_time(args.0, args.1);

        // when the task is complete, push the button
        command_queue.push(MyActionButtonCommand(action_button))
        command_queue
    };

    let action = LazySignals.action::<(f32, f32)>{
        action_fn,
        vec![screen_x, screen_y],
        Vec::<Entity>::new(),
        &mut commands
    }

    // store the reactive entities in a resource to use in systems
    commands.insert_resource(MyConfigResource {
        x_axis,
        y_axis,
        action_button,
        screen_x,
        screen_y,
        log_effect,
        action,
    });
}

fn signals_update_system(config: Res<ConfigResource>, mut commands: Commands) {
    // assume we have read x and y values of the gamepad stick and assigned them to x and y
    let x = ...
    let y = ...

    LazySignals.send(config.x_axis, x, commands);
    LazySignals.send(config.y_axis, y, commands);

    // signals aren't processed right away, so the signals are still the original value
    let prev_x = LazySignals.read::<f32>(config.x_axis, world);
    let prev_y = LazySignals.read::<f32>(config.y_axis, world);

    // let's simulate pressing the action button but use custom send_signal command
    commands.send_signal::<bool>(config.action_button, true);

    // or use our custom local command
    commands.push(MyActionButtonCommand(config.action_button));

    // doing both will only actually trigger the signal once, since multiple calls to send will
    // update the next_value multiple times, but we're lazy, so the signal itself only runs once
    // using whatever value is in next_value when it gets evaluated, i.e. the last signal to
    // actually be sent

    // this is referred to as lossy

    // TODO provide a stream version of signals that provides a Vec<T> instead of Option<T>
    // to the closures

    // in the mean time, if we read x and y and send the signals in the First schedule
    // we can use them to position a sprite during the Update schedule

    // the screen_x and screen_y are only recomputed if the value of x and/or y changed

    // LazySignals.read just returns the data value of the LazySignalsState<f32> component

    // for a Computed, this updates during PreUpdate by default and is otherwise immutable
    // (unless you modify the component directly, which voids the warranty)

    // TODO concrete example using bevy_mod_picking
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // resource to hold the entity ID of each lazy signals primitive
        .init_resource::<ConfigResource>()
        // NOTE: the developer will need to register each custom LazySignalsState<T> type

        // also need to register tuple types for args if they contain custom types (I think)
        // .register_type::<LazyImmutable<MyType>>()

        // f64, i32, bool, &str, and () are already registered

        // add the plugin so the signal processing systems run
        .add_plugins(LazySignalsPlugin)
        .add_systems(Startup, signals_setup_system)
        .add_systems(Update, signals_update_system)
        .run();
}
```

## 🕊 Bevy Compatibility

| bevy   | bevy_lazy_signals |
| ------ | ----------------- |
| 0.14.0 | 0.4.0-alpha       |
| 0.13.2 | 0.3.0-alpha       |

## License

All code in this repository is dual-licensed under either:

- MIT License (LICENSE-MIT or http://opensource.org/licenses/MIT)
- Apache License, Version 2.0 (LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0)

at your option. This means you can select the license you prefer.

## Your contributions

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
