use bevy::prelude::*;

use bevy_lazy_signals::{
    api::LazySignal,
    framework::*,
    reference_impl::make_tuple,
    LazySignalsPlugin,
};

// this just keeps track of all the LazySignals primitives. just need the entity.
#[derive(Resource, Default)]
struct MyTestResource {
    pub signal1: Option<Entity>,
    pub signal2: Option<Entity>,
    pub effect: Option<Entity>,
}

// concrete tuple type to safely work with the DynamicTuple coming out of the LazySignals systems
type MyEffectParams = (Option<bool>, Option<LazySignalsStr>);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // NOTE: the user application will need to register each custom LazyImmutable<T> for reflection
        // .register_type::<LazyImmutable<MyType>>()
        .init_resource::<MyTestResource>()
        // also register type aliases for computed and effect param tuples
        // FIXME can this be done automatically when the Computed or Effect is created?
        .register_type::<MyEffectParams>()
        .add_plugins(LazySignalsPlugin)
        // don't need to add systems to process signals since we're using the plugin
        .add_systems(Startup, init)
        .add_systems(Update, send_some_signals)
        .add_systems(Last, status)
        .run();
}

fn init(mut test: ResMut<MyTestResource>, mut commands: Commands) {
    // create a signal (you need to register data types if not bool, i32, f64, or &str)
    // (see SignalsPlugin)

    // this will derive an Immutable<T> type based on the first parameter type
    // in this case Immutable<bool> is already registered so we're cool
    let test_signal1 = LazySignal.state(false, &mut commands);
    test.signal1 = Some(test_signal1);
    info!("created test signal 1, entity {:?}", test_signal1);

    // for strings the only thing I've gotten to work so far is &'static str
    let test_signal2 = LazySignal.state("test", &mut commands);
    test.signal2 = Some(test_signal2);
    info!("created test signal 2, entity {:?}", test_signal2);

    // simple effect that logs its trigger(s) whenever one changes
    let effect_propagator: Box<dyn EffectFn> = Box::new(|params| {
        // convert DynamicTuple to concrete tuple
        let params = make_tuple::<MyEffectParams>(params);

        // read param 0
        let boolean = params.0.unwrap();

        // read param 1
        let string = params.1.unwrap();

        info!("got {:?} and {:?} from params", boolean, string);
    });

    // trigger an effect from the signal
    test.effect = Some(
        LazySignal.effect::<MyEffectParams>(
            // closure to call when the effect is triggered
            // TODO see if there is some adapter function to wrap an fn that takes tuple as param
            effect_propagator,
            // type of each trigger must match type at same tuple position
            // it's not unsafe; it just won't work if we screw this up
            vec![test_signal1, test_signal2],
            &mut commands
        )
    );
    info!("created test effect, entity {:?}", test.effect);
}

fn send_some_signals(test: Res<MyTestResource>, mut commands: Commands) {
    trace!("sending 'true' to {:?}", test.signal1);
    LazySignal.send(test.signal1, true, &mut commands);
}

fn status(world: &World, test: Res<MyTestResource>) {
    match LazySignal.read::<bool>(test.signal1, world) {
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
