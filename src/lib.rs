use bevy_app::PostUpdate;
use bevy_ecs::{ prelude::*, storage::ComponentSparseSet };

pub mod commands;
use commands::*;

pub mod signals;
use signals::*;

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

/// A reference implementation follows. A consumer can replace any or all pieces and provide a new plugin.

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

impl SignalsResource {
    fn init(&mut self) {
        self.running.clear();
        self.next_running.clear();
        self.processed.clear();
        self.changed.clear();
    }
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
    fn computed<T: Copy + PartialEq + Send + Sync + 'static>(
        propagator: Box<dyn PropagatorFn>,
        sources: Vec<Entity>,
        init_value: T,
        mut commands: Commands
    ) -> Entity {
        let computed = commands.spawn_empty().id();
        commands.create_computed::<T>(computed, propagator, sources, init_value);
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

    fn read<T: Copy + PartialEq + Send + Sync + 'static, Error>(
        immutable: Entity,
        world: &World
    ) -> Result<T, SignalsError> {
        let entity = world.entity(immutable);

        let observable = match entity.get::<LazyImmutable<T>>() {
            Some(observable) => observable,
            None => {
                // TODO maybe add some kind of config option to ignore errors and return default
                return Err(SignalsError::ReadError(immutable));
            }
        };
        Ok(observable.read())
    }

    fn send<T: Copy + PartialEq + Send + Sync + 'static>(
        signal: Entity,
        data: T,
        mut commands: Commands
    ) {
        commands.send_signal::<T>(signal, data);
    }

    fn state<T: Copy + PartialEq + Send + Sync + 'static>(
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
pub fn send_signals(
    world: &mut World,
    query_signals: &mut QueryState<(Entity, &ImmutableComponentId), With<SendSignal>>
) {
    // Phase One:

    // initialize sets
    //signal.init();

    let mut component_id_set = ImmutableComponentSet::new();

    for (entity, immutable) in query_signals.iter(world) {
        component_id_set.insert(entity, immutable.component_id);
    }

    for (entity, component_id) in component_id_set.iter() {
        let mut signal_to_send = world.entity_mut(*entity);

        // mut (apply the next value to) each Immutable
        let mut mut_untyped = signal_to_send.get_mut_by_id(*component_id).unwrap();

        // TODO here we need to access as a LazyMergeable and just run the merge method
        //let lazy_mergeable = mut_untyped as Box<&dyn LazyMergeable>;
        //lazy_mergeable.merge();
        //mut_untyped.map_unchanged(|ptr| unsafe { ptr.deref_mut::<&dyn LazyMergeable>() }).merge();

        // add subscribers to the running set

        // clear subscribers from the current Immutable

        // remove the Signal component
        signal_to_send.remove::<SendSignal>();
    }

    // Phase Two:

    // iterate through a copy of the running set

    // remove an item from the running set

    // skip if already in handled set

    // add the item to the handled set

    // a) item is an effect, so schedule the effect by adding a DeferredEffect component

    // b1) item is a memo, so mark it for recalculation by adding a ComputeMemo component

    // b2) item has its own subscribers, so add those to a new running set

    // loop through the running set until it is empty, then loop through the new running set, and so on
}

pub fn calculate_memos(world: &mut World, query_memos: &mut QueryState<Entity, With<ComputeMemo>>) {
    // need exclusive world access here to update memos immediately and need to write to resource
    world.resource_scope(
        |world, mut signal: Mut<SignalsResource>| {
            // run each Propagator function to recalculate memo, adding sources to the running set

            // *** update the data in the cell

            // add the Memo to the processed set

            // add to the changed set if the value actually changed

            // remove the Memo component

            // merge all next_subscribers sets into subscribers
        }
    );
}

pub fn apply_deferred_effects(
    query_effects: Query<Entity, With<DeferredEffect>>,
    mut signal: ResMut<SignalsResource>,
    mut commands: Commands
) {
    // only run an effect if one of its triggers is in the changed set

    // *** spawn a thread for each effect

    // remove the Effect component
}

/// Plugin to initialize the resource and system schedule.
pub struct SignalsPlugin;

impl bevy_app::Plugin for SignalsPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        // add the systems to process signals, memos, and effects
        app.init_resource::<SignalsResource>()
            //.register_component_as::<dyn LazyMergeable, LazyImmutable<>>()
            .add_systems(
                PostUpdate, // could be Preupdate or whatever else (probably not Update)
                // this ensures each system's changes will be applied before the next is called
                calculate_memos.before(apply_deferred_effects).after(send_signals)
            );
    }
}
