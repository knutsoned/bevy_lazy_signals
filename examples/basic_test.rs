use bevy::prelude::*;

use bevy_signals::{ factory::Signal, signals::EffectFn, SignalsPlugin };

#[derive(Resource, Default)]
struct TestResource {
    pub signal: Option<Entity>,
    pub effect: Option<Entity>,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // NOTE: the user application will need to register each custom Immutable<T> for reflection
        // .register_type::<Immutable<MyType>>()
        .add_plugins(SignalsPlugin)
        .init_resource::<TestResource>()
        .add_systems(Startup, init)
        .add_systems(Update, send_some_signals)
        .add_systems(Last, status)
        .run();
}

fn init(world: &mut World) {
    world.resource_scope(|world, mut test: Mut<TestResource>| {
        let mut commands = world.commands();

        // simple effect as a propagator who logs its triggers whenever one of them changes
        let effect_propagator: Box<dyn EffectFn> = Box::new(|params| {
            info!("running effect {:?}", params);
            Ok(())
        });

        // create a signal (you need to register data types if not bool, i32, f64, or &'static str)
        // (see SignalsPlugin)

        // this will derive an Immutable<T> type based in the first parameter type
        // in this case Immutable<bool> is already registered so we're cool
        let test_signal = Signal.state(false, &mut commands);
        test.signal = Some(test_signal);
        info!("created test signal");

        // trigger an effect from the signal
        test.effect = Some(Signal.effect(effect_propagator, vec![test_signal], &mut commands));
        info!("created test effect");
    });
}

fn send_some_signals(test: Res<TestResource>, mut commands: Commands) {
    trace!("sending 'true' to {:?}", test.signal);
    Signal.send(test.signal, true, &mut commands);
}

fn status(world: &World, test: Res<TestResource>) {
    match Signal.read::<bool>(test.signal, world) {
        Ok(value) => {
            trace!("value: {}", value);
        }
        Err(error) => {
            error!("error: {}", error);
        }
    }
}
