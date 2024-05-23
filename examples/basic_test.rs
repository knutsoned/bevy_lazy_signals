use bevy::prelude::*;

use bevy_lazy_signals::{
    factory::Signal,
    signals::{ get_tuple_from_params, EffectFn },
    SignalsPlugin,
    SignalsStr,
};

#[derive(Resource, Default)]
struct TestResource {
    pub signal1: Option<Entity>,
    pub signal2: Option<Entity>,
    pub effect: Option<Entity>,
}

type EffectParams = (Option<bool>, Option<SignalsStr>);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // NOTE: the user application will need to register each custom Immutable<T> for reflection
        // .register_type::<Immutable<MyType>>()
        // also register type aliases for computed and effect param tuples
        .register_type::<EffectParams>()
        .add_plugins(SignalsPlugin)
        .init_resource::<TestResource>()
        .add_systems(Startup, init)
        .add_systems(Update, send_some_signals)
        .add_systems(Last, status)
        .run();
}

fn init(mut test: ResMut<TestResource>, mut commands: Commands) {
    // create a signal (you need to register data types if not bool, i32, f64, or &'static str)
    // (see SignalsPlugin)

    // this will derive an Immutable<T> type based in the first parameter type
    // in this case Immutable<bool> is already registered so we're cool
    let test_signal1 = Signal.state(false, &mut commands);
    test.signal1 = Some(test_signal1);
    info!("created test signal 1, entity {:?}", test_signal1);

    let test_signal2 = Signal.state("test", &mut commands);
    test.signal2 = Some(test_signal2);
    info!("created test signal 2, entity {:?}", test_signal2);

    // simple effect that logs its trigger(s) whenever one changes
    // TODO try determining the TypeInfo of the params in the system and pass that in
    let effect_propagator: Box<dyn EffectFn> = Box::new(|params| {
        // convert DynamicTuple to concrete tuple
        let params = get_tuple_from_params::<EffectParams>(params);

        // read param 0
        let boolean = params.0;

        // read param 1
        let string = params.1;

        info!("got {:?} and {:?} from params", boolean, string);
    });

    // trigger an effect from the signal
    test.effect = Some(
        Signal.effect::<EffectParams>(
            // closure to call when the effect is triggered
            effect_propagator,
            // type of each trigger must match type at same tuple position
            vec![test_signal1, test_signal2],
            &mut commands
        )
    );
    info!("created test effect, entity {:?}", test.effect);
}

fn send_some_signals(test: Res<TestResource>, mut commands: Commands) {
    trace!("sending 'true' to {:?}", test.signal1);
    Signal.send(test.signal1, true, &mut commands);
}

fn status(world: &World, test: Res<TestResource>) {
    match Signal.read::<bool>(test.signal1, world) {
        Some(Ok(value)) => {
            trace!("value: {}", value);
        }
        Some(Err(error)) => {
            error!("error: {}", error);
        }
        None => {
            trace!("None");
        }
    }
}
