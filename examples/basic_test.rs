use std::time::Duration;

use async_std::task::sleep;
use bevy::{ ecs::world::{ Command, CommandQueue }, prelude::*, tasks::AsyncComputeTaskPool };

use bevy_lazy_signals::{ api::LazySignals, LazySignalsPlugin, StaticStrRef };

// this example toggles a loggged_in value every 10 seconds via an async task, triggering computeds and effects

// simple resource to simulate a service that tracks whether a user is logged in or not
#[derive(Resource, Default)]
struct MyExampleAuthResource {
    logged_in: bool,
}

impl MyExampleAuthResource {
    fn is_logged_in(&self) -> bool {
        self.logged_in
    }
    fn notify_login_status(&mut self, status: bool) {
        self.logged_in = status;
    }
}

// simple command to toggle the login status of the user
struct MyToggleLoginCommand(Entity);

impl Command for MyToggleLoginCommand {
    fn apply(self, world: &mut World) {
        info!("Toggling login");
        if let Some(Ok(status)) = LazySignals.read::<bool>(self.0, world) {
            // it's perfectly ok to return this command in task's queue

            // that would be an infinite loop, but only running once per tick, which may be wanted
            LazySignals.send(self.0, !status, &mut world.commands());
            world.flush_commands();
            info!("...toggled");
        }
    }
}

// this just keeps track of all the LazySignals primitives. just need the entity.
#[derive(Resource, Default)]
struct MyTestResource {
    pub computed: Vec<Entity>,
    pub effect: Vec<Entity>,
    pub signal: Vec<Entity>,
    pub task: Vec<Entity>,
    pub trigger: Vec<Entity>,
}

// concrete tuple type to safely work with the DynamicTuple coming out of the LazySignals systems
type MyClosureArgs = (Option<bool>, Option<StaticStrRef>);

// making an alias to make it easier to read code in some places
type MyAuthArgs = MyClosureArgs;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // resource to simulate something external to update
        .init_resource::<MyExampleAuthResource>()
        // resource to hold the entity ID of each lazy signals primitive
        .init_resource::<MyTestResource>()
        // add the plugin so the signal processing systems run
        .add_plugins(LazySignalsPlugin)
        // add our app-specific systems
        .add_systems(Startup, init)
        .add_systems(Update, send_some_signals)
        .add_systems(Last, status)
        .run();
}

fn init(mut test: ResMut<MyTestResource>, mut commands: Commands) {
    // create a signal (you need to register data types if not bool, i32, f64, or &'static str)
    // (see LazySignalsPlugin)

    // this will reflect a LazySignalsState<T> type based on the provided concrete T

    // in this case LazySignalsState<bool> is already registered so we're cool

    // in this example, signal0 would be sent whenever a user logs in or logs out
    let signal0 = LazySignals.state(false, &mut commands);

    // leave signals and computeds as local values to use as deps throughout the init system
    // since we can't move the deps into our closures from the test resource
    test.signal.push(signal0);
    info!("created test signal 0, entity {:#?}", test.signal[0]);

    // for strings the only thing I've gotten to work so far is &'static str
    let signal1 = LazySignals.state("Congrats, you logged in somehow", &mut commands);
    test.signal.push(signal1);
    info!("created test signal 1, entity {:#?}", test.signal[1]);

    // for an effect trigger, we don't care about the value, only that the trigger signal was sent

    // we could use a regular signal but something like a button click might not need a type

    // for a basic trigger, we use LazySignalsState<()> as the signal component type

    // there's also a way to send a regular signal as a trigger but beware: that is a good recipe
    // for an update storm

    // it's fine if you're triggering an effect, but the send_and_trigger API call will update a
    // Signal's value and then force every Computed down it's subscriber tree to recompute and
    // trigger all effects in the tree

    // this would be desired if a value isn't changing but the effects still need triggering

    // sometimes it may be easier and beneficial to do that, but just beware that all the Computeds
    // will run even if none of the values changed

    // note the above refers to adding a signal as a source but then using send_and_trigger to send
    // even when the data has not changed

    // using a signal with data in the trigger vec already does that, although the data is not
    // passed in as a source if it's in the trigger vec, so that won't trigger a computation unless
    // the data actually changes and is used as a source somewhere

    // TODO make sure send_and_trigger works the way we think it does
    let trigger0 = LazySignals.state((), &mut commands);
    test.trigger.push(trigger0);
    info!("created test trigger 0, entity {:#?}", test.trigger[0]);

    // simple effect that logs its sources whenever one changes or it is triggered
    let log_logins = |args: MyClosureArgs, world: &mut World| {
        // read arg 0
        if let Some(logged_in) = args.0 {
            info!("EFFECT0: got {} from args.0, updating resource", logged_in);
            // we have exclusive world access. in this case, we update a value in a resource
            world.resource_scope(
                // if this closure needs to invoke an external library accessed via
                // MyExampleAuthResource (e.g. C or C++ FFI calls)
                // may need to get it as a NonSendMut for thread safety
                |_world, mut example_auth_resource: Mut<MyExampleAuthResource>| {
                    // keep our resource in sync with our signal
                    example_auth_resource.notify_login_status(logged_in);
                }
            );
        }

        // read arg 1
        if let Some(logged_in_msg) = args.1 {
            info!("EFFECT0: got {} from args.1", logged_in_msg);
        }
    };

    // we can just push this into the resource since we don't need to pass it around as a dep
    test.effect.push(
        // MyClosureArgs == args tuple type
        LazySignals.effect::<MyClosureArgs>(
            // closure to call when the effect is triggered
            log_logins,
            // type of each source must match type at same tuple position
            // it's not unsafe(?); it just won't work if we screw this up
            // TODO definitely think about that some more
            vec![signal0, signal1], // sending either signal triggers the effect
            // explicit triggers are not added to the args tuple like sources are
            Vec::<Entity>::new(),
            &mut commands
        )
    );

    info!("created test effect 0, entity {:#?}", test.effect[0]);

    // simple closure that shows a supplied value or an error message

    // this closure could be used multiple times with different entities holding the memoized value
    // and different sources, but we have to specify the args type here and it has to match when we
    // actually make each Computed
    let derive_login_msg = |args: MyAuthArgs| {
        // here we are specifically using the MyAuthArgs alias to make it easier to tell what
        // these args are for, at the expense of making it easier to find the main definition

        // MyAuthArgs, MyClosureArgs, (Option<bool>, Option<LazySignalsStr>), and
        // (Option<bool>, Option<&str>) are interchangeable when defining computeds and effects

        // default error message (only if args.0 == false)
        let mut value = "You are not authorized to view this";

        // if logged_in
        // (Err or None will return Err or None, this block runs only if args.0 == true)
        if args.0? {
            // show a logged in message, if one exists
            if let Some(msg) = args.1 {
                value = msg;
            } else {
                value = "Greetings, Starfighter";
            }

            // could also just do: let value = args.1?;
            // and bubble the error up as a None return value

            // the fn would return right away and the next lines would not run
        }

        info!("COMPUTED0 value: {}", value);
        Some(Ok(value))
    };

    // simple computed to store the string value or an error, depending on the bool

    // MyClosureArgs == args tuple type, StaticStrRef (&'static str) == return type
    let computed0 = LazySignals.computed::<MyAuthArgs, StaticStrRef>(
        derive_login_msg,
        vec![signal0, signal1], // sending either signal triggers a recompute
        &mut commands
    );
    test.computed.push(computed0);
    info!("created test computed 0, entity {:#?}", test.computed[0]);

    // simple computed to store a string value from a computed, or an error, depending on the bool
    let computed1 = LazySignals.computed::<MyAuthArgs, StaticStrRef>(
        |args| {
            // default error message
            let mut value = "You are not authorized to view this";

            // if logged_in
            if let Some(logged_in) = args.0 {
                if logged_in {
                    // show a logged in message, if one exists
                    if let Some(msg) = args.1 {
                        value = msg;
                    }
                }
            }

            info!("COMPUTED1 value: {}", value);
            Some(Ok(value))
        },
        vec![signal0, computed0],
        &mut commands
    );
    test.computed.push(computed1);
    info!("created test computed 1, entity {:#?}", test.computed[1]);

    // set this one up to get the msg from a memo instead of a signal
    test.effect.push(
        LazySignals.effect::<MyAuthArgs>(
            // closure to call when the effect is triggered
            |args, _world| {
                // second effect, same as the first, but use the memo as the string instead of the signal

                // read arg 0
                if let Some(logged_in) = args.0 {
                    info!("EFFECT1: got logged_in: {}", logged_in);
                }

                // read arg 1
                if let Some(logged_in_msg) = args.1 {
                    info!("EFFECT1: got logged_in_msg: {}", logged_in_msg);
                }
            },
            vec![signal0, computed1],
            Vec::<Entity>::new(),
            &mut commands
        )
    );
    info!("created test effect 1, entity {:#?}", test.effect[1]);

    // set up a long-running async task with triggers only and no sources (pass in unit type)

    // there's no reason a task can't take args. the closure fn sig is the same as an effect except
    // a task does not have direct world access

    // it can add commands to a queue only and the queue will run when the system runs to check it
    // and the task has returned the queue
    test.task.push(
        LazySignals.task::<()>(
            // closure to call when triggered
            move |_args| {
                let thread_pool = AsyncComputeTaskPool::get();
                thread_pool.spawn(async move {
                    info!("\nTASK0: triggered");
                    let mut command_queue = CommandQueue::default();

                    // stand by 10 seconds for station identification
                    sleep(Duration::from_secs(10)).await;

                    info!("TASK0: done sleeping");

                    // even triggered continuously the task will only run once the prior task exits

                    // even if this task runs multiple times in the same tick, it is idempotent
                    // at least within that tick, because it reads the immutable value and
                    // the value of the sent signal is relative to that

                    // all the tasks that return their commands in the same tick would then send
                    // the same signal, which would update the LazySignalsState component with a
                    // next_value several times, but only result in sending the signal once

                    // signal0 could be a dependency of the task and that would be ok

                    // we would get an infinite loop but no update storm because the update is
                    // batched and not immediate
                    command_queue.push(MyToggleLoginCommand(signal0));

                    command_queue
                })
            },
            Vec::<Entity>::new(),
            // triggering a signal will run effects without passing the signal's value as a param
            // (it still sends the value of the sources as usual, although this task has none)
            vec![trigger0],
            &mut commands
        )
    );
    info!("created test task 0, entity {:#?}", test.task[0]);

    info!("init complete");
}

fn send_some_signals(test: Res<MyTestResource>, mut commands: Commands) {
    /* uncomment this to automatically log the user back in on the next tick after logging out
    trace!("sending 'true' to {:?}", test.signal1);
    LazySignals.send(test.signal1, true, &mut commands);
    */

    // even though this runs every tick, the task will trigger once, then run until it exits
    // before being eligible to run again
    trace!("triggering {:?}", test.trigger[0]);
    LazySignals.trigger(test.trigger[0], &mut commands);
}

fn status(
    world: &World,
    example_auth_resource: Res<MyExampleAuthResource>,
    test: Res<MyTestResource>
) {
    trace!("logged in: {}", example_auth_resource.is_logged_in());

    match LazySignals.read::<bool>(test.signal[0], world) {
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
