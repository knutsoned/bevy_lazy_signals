use bevy::{ prelude::*, reflect::{ DynamicTuple, Tuple } };

use bevy_signals::{ factory::Signal, signals::EffectFn, SignalsPlugin, SignalsStr };

#[derive(Resource, Default)]
struct TestResource {
    pub signal1: Option<Entity>,
    pub signal2: Option<Entity>,
    pub effect: Option<Entity>,
}

type EffectParams = (bool, SignalsStr);

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

fn get_tuple<T: Tuple>(params: &DynamicTuple) -> Option<&T> {
    params.as_reflect().downcast_ref::<T>()
}

fn init(world: &mut World) {
    world.resource_scope(|world, mut test: Mut<TestResource>| {
        let mut commands = world.commands();

        // simple effect that logs its trigger(s) whenever one changes
        // TODO try determining the TypeInfo of the params in the system and pass that in
        let effect_propagator: Box<dyn EffectFn> = Box::new(|params: DynamicTuple| {
            //params.set_represented_type(T.type_info());
            let params = get_tuple::<EffectParams>(&params);
            info!("running effect with {:?}", params);

            // TODO read param 0

            // TODO read param 1

            // TODO something with those values

            Ok(())
        });

        // create a signal (you need to register data types if not bool, i32, f64, or &'static str)
        // (see SignalsPlugin)

        // this will derive an Immutable<T> type based in the first parameter type
        // in this case Immutable<bool> is already registered so we're cool
        let test_signal1 = Signal.state(false, &mut commands);
        test.signal1 = Some(test_signal1);
        info!("created test signal 1");

        let test_signal2 = Signal.state("true", &mut commands);
        test.signal2 = Some(test_signal2);
        info!("created test signal 2");

        // trigger an effect from the signal
        test.effect = Some(
            Signal.effect(effect_propagator, vec![test_signal1, test_signal2], &mut commands)
        );
        info!("created test effect");
    });
}

fn send_some_signals(test: Res<TestResource>, mut commands: Commands) {
    trace!("sending 'true' to {:?}", test.signal1);
    Signal.send(test.signal1, true, &mut commands);
}

fn status(world: &World, test: Res<TestResource>) {
    match Signal.read::<bool>(test.signal1, world) {
        Ok(value) => {
            trace!("value: {}", value);
        }
        Err(error) => {
            error!("error: {}", error);
        }
    }
}
