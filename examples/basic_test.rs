use bevy::prelude::*;

use bevy_lazy_signals::{ api::LazySignals, framework::*, LazySignalsPlugin };

// simple resource to simulate a service that tracks whether a user is logged in or not
#[derive(Resource, Default)]
struct MyExampleAuthResource {
    logged_in: bool,
}
impl MyExampleAuthResource {
    fn is_logged_in(&self) -> bool {
        self.logged_in
    }
    fn notify_logged_in(&mut self) {
        self.logged_in = true;
    }
    fn notify_logged_out(&mut self) {
        self.logged_in = false;
    }
}

// this just keeps track of all the LazySignals primitives. just need the entity.
#[derive(Resource, Default)]
struct MyTestResource {
    pub computed1: Option<Entity>,
    pub _computed2: Option<Entity>,
    pub effect1: Option<Entity>,
    pub _effect2: Option<Entity>,
    pub signal1: Option<Entity>,
    pub signal2: Option<Entity>,
}

// concrete tuple type to safely work with the DynamicTuple coming out of the LazySignals systems
type MyClosureParams = (Option<bool>, Option<LazySignalsStr>);

// you only have to register the main definition, not aliases like this one.
type MyAuthParams = MyClosureParams;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // resource to simulate something external to update
        .init_resource::<MyExampleAuthResource>()
        // resource to hold the entity ID of each lazy signals primitive
        .init_resource::<MyTestResource>()
        // NOTE: the user application will need to register each custom LazyImmutable<T> for reflection
        // .register_type::<LazyImmutable<MyType>>()
        // also register type aliases for computed and effect param tuples
        // FIXME can this be done automatically when the Computed or Effect is created?
        .register_type::<MyClosureParams>()
        // add the plugin so the signal processing systems run
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

    // in this example, signal1 is sent whenever a user logs in or logs out
    let test_signal1 = LazySignals.state(false, &mut commands);
    test.signal1 = Some(test_signal1);
    info!("created test signal 1, entity {:?}", test_signal1);

    // for strings the only thing I've gotten to work so far is &str
    // (usually &'static str but just &str if used as a PropagatorFn result type)
    let test_signal2 = LazySignals.state("Congrats, you logged in somehow", &mut commands);
    test.signal2 = Some(test_signal2);
    info!("created test signal 2, entity {:?}", test_signal2);

    // simple effect that logs its trigger(s) whenever one changes
    let effect1_fn: Box<dyn Effect<MyClosureParams>> = Box::new(|params, world| {
        // read param 0
        let logged_in = params.0.unwrap();

        // read param 1
        let logged_in_msg = params.1.unwrap();

        info!("got {} and {} from params", logged_in, logged_in_msg);

        // we have exclusive world access. in this case, we update a value in a resource
        world.resource_scope(|_world, mut example_auth_resource: Mut<MyExampleAuthResource>| {
            // keep our resource in sync with our signal
            if logged_in {
                example_auth_resource.notify_logged_in()
            } else {
                example_auth_resource.notify_logged_out()
            }
        });
    });

    // trigger an effect from the signal
    test.effect1 = Some(
        LazySignals.effect::<MyClosureParams>(
            // closure to call when the effect is triggered
            effect1_fn,
            // type of each trigger must match type at same tuple position
            // it's not unsafe; it just won't work if we screw this up
            vec![test_signal1, test_signal2], // sending either signal triggers the effect
            &mut commands
        )
    );
    info!("created test effect 1, entity {:?}", test.effect1);

    // simple closure that shows a supplied value or an error message

    // this closure could be used multiple times with different entities holding the memoized value
    // and different sources
    let computed1_fn: Box<dyn Propagator<MyAuthParams, &str>> = Box::new(|params| {
        // here we are specifically using the MyAuthParams alias to make it easier to tell what
        // these params are for, at the expense of making it easier to find the main definition

        // MyAuthParams, MyClosureParams, (Option<bool>, Option<LazySignalsStr>), and
        // (Option<bool>, Option<&str>) are interchangeable when defining propagators and effects

        // default error message
        let mut value = "You are not authorized to view this";

        // if loggedIn
        if let Some(show) = params.0 {
            if show {
                // show a logged in message, if one exists
                if let Some(msg) = params.1 {
                    value = msg;
                } else {
                    value = "Greetings, Starfighter";
                }
            }
        }

        info!("computed1 value: {}", value);
        Some(value)
    });

    // simple computed to store the string value or an error, depending on the bool
    let test_computed1 = LazySignals.computed::<MyAuthParams, &str>(
        computed1_fn,
        vec![test_signal1, test_signal2], // sending either signal triggers a recompute
        &mut commands
    );
    test.computed1 = Some(test_computed1);

    info!("init complete");
}

fn send_some_signals(test: Res<MyTestResource>, mut commands: Commands) {
    trace!("sending 'true' to {:?}", test.signal1);
    LazySignals.send(test.signal1, true, &mut commands);
}

fn status(
    world: &World,
    example_auth_resource: Res<MyExampleAuthResource>,
    test: Res<MyTestResource>
) {
    trace!("logged in: {}", example_auth_resource.is_logged_in());

    match LazySignals.read::<bool>(test.signal1, world) {
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
