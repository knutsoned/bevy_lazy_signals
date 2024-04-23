/*
use bevy_app::PostUpdate;
*/
use bevy_ecs::prelude::*;
use prelude::PropagatorFn;

pub mod signals;
use signals::*;

pub mod prelude {
    pub use crate::signals::*;
}

#[derive(Resource)]
struct ReactiveContext {
    // sparse sets we will need: running, executed, changed
}

pub struct SignalsPlugin;

impl SignalsPlugin {
    /// ## Systems
    /// These systems are meant to be run in tight sequence, preferably as a chain
    pub fn signals_system(query_signals: Query<Entity, With<SendSignal>>, world: &mut World) {
        // Phase One:

        // *** apply the next value to each Immutable

        // add subscribers to the running set

        // clear subscribers from the current Immutable

        // remove the Signal component

        // Phase Two:

        // iterate through a copy of the running set

        // remove an item from the running set

        // skip if already in handled set

        // add the item to the handled set

        // a) item is an effect, so schedule the effect by adding an Effect component

        // b1) item is a memo, so mark it for recalculation
        // --- or maybe just recalculate immediately so we don't need memos_system ---

        // b2) item has its own subscribers, so add those to the running set

        // loop through the running set until it is empty
    }

    pub fn memos_system(query_memos: Query<Entity, With<ComputeMemo>>, world: &mut World) {
        // run each Propagator function to recalculate memo

        // *** update the data in the cell

        // remove the Memo component

        // merge all next_subscribers sets into subscribers
    }

    pub fn effects_system(
        query_effects: Query<Entity, With<DeferredEffect>>,
        commands: &mut Commands,
        world: &World
    ) {
        // *** spawn a thread for each effect

        // remove the Effect component
    }
}

impl Signal for SignalsPlugin {
    fn computed(
        propagator: Box<dyn PropagatorFn>,
        sources: Vec<Entity>,
        world: &mut World
    ) -> Entity {
        todo!()
    }

    fn effect(
        propagator: Box<dyn PropagatorFn>,
        triggers: Vec<Entity>,
        world: &mut World
    ) -> Entity {
        todo!()
    }

    fn send<T>(next_value: T, world: &mut World) {
        todo!()
    }

    fn state<T>(value: T, world: &mut World) -> Entity {
        todo!()
    }

    fn value<T>(immutable: Entity, world: &mut World) -> T {
        todo!()
    }
}

impl bevy_app::Plugin for SignalsPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        // TODO add the systems to process signals, memos, and effects
        /*
        app.init_resource::<ReactiveContext<World>>().add_systems(
            PostUpdate,
            Self::apply_deferred_effects
        );
        */
    }
}
