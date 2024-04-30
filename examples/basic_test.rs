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
        .add_systems(Startup, setup)
        .add_systems(Update, send_some_signals)
        .run();
}

fn setup(mut test: ResMut<TestResource>, mut commands: Commands) {
    let effect_propagator: &PropagatorFn = &(|_world, triggers, target| {
        info!("triggers: {:?}", triggers);
        if target.is_some() {
            error!("effects should not have targets!");
        }
    });
    test.signal = Some(Signal.state(false, &mut commands));
    test.effect = Some(Signal.effect(effect_propagator, vec![test.signal], &mut commands));
}

fn send_some_signals(test: ResMut<TestResource>, mut commands: Commands) {
    Signal.send(test.signal.unwrap(), true, &mut commands);
}
