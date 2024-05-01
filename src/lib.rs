use bevy_app::PostUpdate;
use bevy_ecs::prelude::*;
use bevy_reflect::*;
use bevy_utils::tracing::*;

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
}

/// A reference implementation follows. A consumer can replace any or all pieces and provide a new plugin.
///
/// Shared reactive context resource.
#[derive(Resource)]
pub struct SignalsResource {
    /// Tracks the currently running iteration (immutable once the iteration starts).
    pub running: EntitySet,

    /// Tracks what will run after the end of the current iteration.
    pub next_running: EntitySet,

    /// Tracks which memos have already been added to a running set.
    pub processed: EntitySet,

    /// Tracks which Signals and Memos actually have changed data.
    pub changed: EntitySet,

    /// Tracks Effects to evaluate for processing.
    pub deferred: EntitySet,

    /// Tracks Effects that are still running and should not be re-triggered.
    pub effects: EntitySet,
}

impl SignalsResource {
    fn init(&mut self) {
        self.running.clear();
        self.next_running.clear();
        self.processed.clear();
        self.changed.clear();
        self.deferred.clear();
        // self.effects.clear(); // don't clear this, it needs to remember what is going on
    }
}

impl Default for SignalsResource {
    fn default() -> Self {
        Self {
            running: empty_set(),
            next_running: empty_set(),
            processed: empty_set(),
            changed: empty_set(),
            deferred: empty_set(),
            effects: empty_set(),
        }
    }
}

pub type SignalsResult<T> = Result<T, SignalsError>;

pub struct Signal;

/// This is the reference user API. patterned after the TC39 proposal.
impl Signal {
    pub fn computed<T: Copy + PartialEq + Send + Sync + 'static>(
        &self,
        propagator: &'static PropagatorFn,
        sources: Vec<Entity>,
        init_value: T,
        commands: &mut Commands
    ) -> Entity {
        let computed = commands.spawn_empty().id();
        commands.create_computed::<T>(computed, propagator, sources, init_value);
        computed
    }

    pub fn effect(
        &self,
        propagator: &'static PropagatorFn,
        triggers: Vec<Entity>,
        commands: &mut Commands
    ) -> Entity {
        let effect = commands.spawn_empty().id();
        commands.create_effect(effect, propagator, triggers);
        effect
    }

    pub fn read<T: Copy + PartialEq + Send + Sync + 'static>(
        &self,
        immutable: Option<Entity>,
        world: &World
    ) -> SignalsResult<T> {
        if immutable.is_none() {
            return Err(SignalsError::NoSignalError);
        }
        let immutable = immutable.unwrap();
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

    pub fn send<T: Copy + PartialEq + Send + Sync + 'static>(
        &self,
        signal: Entity,
        data: T,
        commands: &mut Commands
    ) {
        commands.send_signal::<T>(signal, data);
    }

    pub fn state<T: Copy + PartialEq + Send + Sync + 'static>(
        &self,
        data: T,
        commands: &mut Commands
    ) -> Entity {
        let state = commands.spawn_empty().id();
        commands.create_state::<T>(state, data);
        state
    }
}

/// ## Systems
/// These systems are meant to be run in tight sequence, preferably like the plugin demonstrates.
/// Any commands in each system must be applied before proceeding to the next.
pub fn send_signals(
    world: &mut World,
    query_signals: &mut QueryState<(Entity, &ImmutableComponentId), With<SendSignal>>
) {
    info!("SIGNALS");

    // Phase One:
    world.resource_scope(|world, mut signal: Mut<SignalsResource>| {
        // initialize sets
        signal.init();

        let mut count = 0;
        let mut component_id_set = ComponentIdSet::new();
        let mut component_info = ComponentInfoSet::new();

        info!("looking for signals");
        // build component id -> info map
        for (entity, immutable) in query_signals.iter(world) {
            let component_id = immutable.component_id;
            info!("found a signal for component ID {:?}", component_id);
            component_id_set.insert(entity, component_id);
            if let Some(info) = world.components().get_info(component_id) {
                component_info.insert(component_id, info.clone());
            }
            count += 1;
        }
        info!("found {} signals to send", count);

        // build reflect types for merge operation on reflected UntypedObservable trait object
        world.resource_scope(|world, type_registry: Mut<AppTypeRegistry>| {
            for (entity, component_id) in component_id_set.iter() {
                let mut signal_to_send = world.entity_mut(*entity);

                // here we need to access the Signal as an UntypedObservable and just run the merge method
                let type_registry = type_registry.read();

                // use the type_id from the component info, YOLO
                if let Some(info) = component_info.get(*component_id) {
                    if let Some(type_id) = info.type_id() {
                        let reflect_data = type_registry.get(type_id).unwrap();
                        let reflect_from_ptr = reflect_data.data::<ReflectFromPtr>().unwrap();

                        // mut (apply the next value to) each Immutable
                        let mut mut_untyped = signal_to_send.get_mut_by_id(*component_id).unwrap();
                        let ptr = mut_untyped.as_mut();

                        // SAFE: `value` implements reflected trait `UntypedObservable`, which the `ReflectFromPtr` was created for
                        let value = unsafe { reflect_from_ptr.as_reflect_mut(ptr) };

                        let reflect_merge = type_registry
                            .get_type_data::<ReflectUntypedObservable>(value.type_id())
                            .unwrap();
                        let untyped_observable: &mut dyn UntypedObservable = reflect_merge
                            .get_mut(value)
                            .unwrap();

                        let subs = untyped_observable.merge();
                        info!("adding {:?} subs", subs);

                        // add subscribers to the running set
                        for subscriber in subs.into_iter() {
                            signal.running.insert(subscriber, ());
                            info!("added subscriber {:?} into running set", subscriber);
                        }
                    }
                }

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
        });
    });
}

pub fn calculate_memos(world: &mut World, query_memos: &mut QueryState<Entity, With<ComputeMemo>>) {
    info!("MEMOS");
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
    info!("EFFECTS");
    // only run an effect if one of its triggers is in the changed set

    // *** spawn a thread for each effect

    // remove the Effect component
}

/// Plugin to initialize the resource and system schedule.
pub struct SignalsPlugin;

impl bevy_app::Plugin for SignalsPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        // NOTE: the user application will need to register each custom Immutable<T> for reflection

        // add the systems to process signals, memos, and effects
        app.init_resource::<SignalsResource>()
            // custom Immutable types must be manually registered
            .register_type::<ImmutableBool>()
            .register_type::<ImmutableInt>()
            .register_type::<ImmutableFloat>()
            .register_type::<ImmutableStr>()
            //.register_component_as::<dyn LazyMergeable, LazyImmutable<>>()
            .add_systems(
                PostUpdate, // could be Preupdate or whatever else (probably not Update)
                // this ensures each system's changes will be applied before the next is called
                (
                    send_signals.before(calculate_memos),
                    calculate_memos.before(apply_deferred_effects),
                    apply_deferred_effects,
                )
            );
    }
}
