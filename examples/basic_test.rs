use bevy::prelude::*;

use bevy_signals::{ signals::PropagatorFn, Signal, SignalsPlugin };

#[derive(Resource, Default)]
struct TestResource {
    pub signal: Option<Entity>,
    pub effect: Option<Entity>,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // NOTE: the user application will need to register each custom Immutable<T> for reflection
        .add_plugins(SignalsPlugin)
        .init_resource::<TestResource>()
        .add_systems(Startup, init)
        .add_systems(Update, send_some_signals)
        .add_systems(Last, status)
        .run();
}

fn init(mut test: ResMut<TestResource>, mut commands: Commands) {
    // simple effect
    let effect_propagator: &PropagatorFn = &(|_world, triggers, target| {
        info!("triggers: {:?}", triggers);
        if target.is_some() {
            error!("effects should not have targets!");
        }
    });

    // create a signal (you need to register data types if not bool, i32, f64, or &'static str)
    // see SignalsPlugin
    let test_signal = Signal.state(false, &mut commands);
    test.signal = Some(test_signal);
    info!("created test signal");

    // trigger an effect from the signal
    test.effect = Some(Signal.effect(effect_propagator, vec![test_signal], &mut commands));
    info!("created test effect");
}

fn send_some_signals(test: ResMut<TestResource>, mut commands: Commands) {
    trace!("sending 'true' to {:?}", test.signal);
    Signal.send(test.signal, true, &mut commands);
}

fn status(world: &mut World) {
    world.resource_scope(|world, test: Mut<TestResource>| {
        match Signal.read::<bool>(test.signal, world) {
            Ok(value) => {
                trace!("value: {}", value);
            }
            Err(error) => {
                error!("error: {}", error);
            }
        }
    });
}
