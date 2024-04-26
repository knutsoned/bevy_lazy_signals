use bevy_app::PostUpdate;
use bevy_ecs::prelude::*;

pub mod commands;
use commands::*;

pub mod signals;
use signals::*;

mod utilities;
use utilities::*;

pub mod prelude {
    pub use crate::{
        SignalsPlugin,
        SignalsResource,
        apply_deferred_effects,
        calculate_memos,
        send_signals,
    };
    pub use crate::commands::*;
    pub use crate::signals::*;
}

/// A reference implementation follows. A consumer can replace any or all and provide a new plugin.

/// Shared reactive context resource.
#[derive(Resource)]
pub struct SignalsResource {
    /// Tracks the currently running iteration (immutable once the iteration starts).
    pub running: EntitySet,

    /// Tracks what will run after the end of the current iteration.
    pub next_running: EntitySet,

    /// Tracks what has already been added to a running set.
    pub processed: EntitySet,

    /// Tracks which Signals and Memos actually have changed data.
    pub changed: EntitySet,
}

impl Default for SignalsResource {
    fn default() -> Self {
        Self {
            running: empty_set(),
            next_running: empty_set(),
            processed: empty_set(),
            changed: empty_set(),
        }
    }
}

impl Signal for SignalsResource {
    fn computed<T: Copy + Default + PartialEq + Send + Sync + 'static>(
        propagator: Box<dyn PropagatorFn>,
        sources: Vec<Entity>,
        mut commands: Commands
    ) -> Entity {
        let computed = commands.spawn_empty().id();
        commands.create_computed::<T>(computed, propagator, sources);
        computed
    }

    fn effect(
        propagator: Box<dyn PropagatorFn>,
        triggers: Vec<Entity>,
        mut commands: Commands
    ) -> Entity {
        let effect = commands.spawn_empty().id();
        commands.create_effect(effect, propagator, triggers);
        effect
    }

    fn read<T: Copy + Default + PartialEq + Send + Sync + 'static>(
        immutable: Entity,
        world: &World
    ) -> T {
        let mut value = T::default();
        let immutable = world.entity(immutable);

        // TODO should this panic instead?
        if let Some(observable) = immutable.get::<LazyImmutable<T>>() {
            value = observable.read();
        }
        value
    }

    fn send<T: Copy + Default + PartialEq + Send + Sync + 'static>(
        signal: Entity,
        data: T,
        mut commands: Commands
    ) {
        commands.send_signal::<T>(signal, data);
    }

    fn state<T: Copy + Default + PartialEq + Send + Sync + 'static>(
        data: T,
        mut commands: Commands
    ) -> Entity {
        let state = commands.spawn_empty().id();
        commands.create_state::<T>(state, data);
        state
    }
}

/// ## Systems
/// These systems are meant to be run in tight sequence, preferably like the plugin demonstrates.
/// The commands in the first system must be applied before proceeding to the other two.
pub fn send_signals(query_signals: Query<Entity, With<SendSignal>>, commands: Commands) {
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

    // b2) item has its own subscribers, so add those to a new running set

    // loop through the running set until it is empty, then loop through the new running set, and so on

}

pub fn calculate_memos(world: &mut World, query_memos: &mut QueryState<Entity, With<ComputeMemo>>) {
    // run each Propagator function to recalculate memo

    // *** update the data in the cell

    // remove the Memo component

    // merge all next_subscribers sets into subscribers
}

pub fn apply_deferred_effects(
    world: &mut World,
    query_effects: &mut QueryState<Entity, With<DeferredEffect>>
) {
    // *** spawn a thread for each effect

    // remove the Effect component
}

/// Plugin to initialize the resource and system schedule.
pub struct SignalsPlugin;

impl bevy_app::Plugin for SignalsPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        // add the systems to process signals, memos, and effects
        app.init_resource::<SignalsResource>().add_systems(
            PostUpdate, // could be Preupdate or whatever else (probably not Update)
            // this ensures each system's changes will be applied before the next is called
            calculate_memos.before(apply_deferred_effects).after(send_signals)
        );
    }
}
