pub mod computed;
pub mod effect;
pub mod init;
pub mod signal;

use std::{ any::TypeId, sync::RwLockReadGuard };

use bevy::{ ecs::component::ComponentId, prelude::*, reflect::TypeRegistry };

use crate::{ arcane_wizardry::*, framework::* };

/// These are the reference user API systems, patterned after the TC39 proposal.
///
/// Shared reactive context resource, aka global state.
/// This tracks long-running effects across ticks but otherwise should start fresh each cycle.
/// Main purpose is to provide "stack"-like functionality across systems in the processing chain.
#[derive(Resource)]
pub struct LazySignalsResource {
    /// Tracks triggered entities (notify subscribers even if the value did not change).
    pub triggered: EntitySet,

    /// Tracks the currently running iteration (immutable once the iteration starts).
    /// Used during signal sending.
    pub running: EntitySet,

    /// Tracks what will run after the end of the current iteration.
    /// Used during signal sending.
    pub next_running: EntitySet,

    /// Tracks which memos have already been added to a running set.
    /// Used during signal sending.
    pub processed: EntitySet,

    /// Tracks the currently running computation.
    pub compute_stack: Vec<Entity>,

    /// Tracks which Signals and Memos actually have changed data.
    pub changed: EntitySet,

    /// Tracks Effects to evaluate for processing.
    pub deferred: EntitySet,

    /// Tracks Effects that are still running and should not be re-triggered.
    pub effects: EntitySet,

    /// Tracks errors that occur when things try to run.
    pub errors: ErrorSet,
}

/// This is a singleton that represents the "global state." It is used during internal updates.
impl LazySignalsResource {
    /// Call this at the start of each run to make sure everything is fresh.
    fn init(&mut self) {
        self.triggered.clear();
        self.running.clear();
        self.next_running.clear();
        self.processed.clear();
        self.compute_stack.clear();
        self.changed.clear();
        self.deferred.clear();
        // self.effects.clear(); // don't clear this, need.. to remember... what is going on
        self.errors.clear();
    }

    // if there is a next_running set, move it into the running set and empty it
    pub fn merge_running(&mut self) -> bool {
        if self.next_running.is_empty() {
            false
        } else {
            for index in self.next_running.indices() {
                self.running.insert(index, ());
            }
            self.next_running.clear();
            true
        }
    }
}

impl Default for LazySignalsResource {
    fn default() -> Self {
        Self {
            triggered: empty_set(),
            running: empty_set(),
            next_running: empty_set(),
            processed: empty_set(),
            compute_stack: Vec::new(),
            changed: empty_set(),
            deferred: empty_set(),
            effects: empty_set(),
            errors: ErrorSet::new(),
        }
    }
}

fn add_subs_to_hierarchy(
    query_effects: &mut QueryState<(Entity, &LazyEffect), With<DeferredEffect>>,
    hierarchy: &mut EntityHierarchySet,
    subs_closure: Box<dyn EffectSubsFn>,
    world: &mut World
) {
    for (entity, effect) in query_effects.iter(world) {
        let subs = hierarchy.get_or_insert_with(entity, || { Vec::new() });
        subs.append(&mut subs_closure(effect));
    }
}

fn add_subs_to_running(subs: &[Entity], triggered: bool, signals: &mut Mut<LazySignalsResource>) {
    // add subscribers to the next running set
    for subscriber in subs.iter() {
        let subscriber = *subscriber;
        signals.next_running.insert(subscriber, ());
        info!("-added subscriber {:?} to running set", subscriber);

        // if these subs were triggered, they need to be marked triggered too
        if triggered {
            // add each one to the triggered set
            signals.triggered.insert(subscriber, ());
        }
    }
}

/// Convenience function to subscribe an entity to a source.
fn subscribe(
    world: &mut World,
    entity: &Entity,
    source: &Entity,
    type_registry: &RwLockReadGuard<TypeRegistry>
) {
    // get the TypeId of each source (Signal or Memo) Immutable component
    let mut component_id: Option<ComponentId> = None;
    let mut type_id: Option<TypeId> = None;

    info!("Subscribing {:#?} to {:?}", entity, source);
    // get a readonly reference to the source entity
    if let Some(source) = world.get_entity(*source) {
        info!("-got source EntityRef");
        // get the source Immutable component
        if let Some(immutable_state) = source.get::<ImmutableState>() {
            info!("-got ImmutableState");
            // ...as a SignalsObservable
            component_id = Some(immutable_state.component_id);
            if let Some(info) = world.components().get_info(component_id.unwrap()) {
                info!("-got TypeId");
                type_id = info.type_id();
            }
        }
    }

    // we have a component and a type, now do mutable stuff
    if component_id.is_some() && type_id.is_some() {
        if let Some(mut source) = world.get_entity_mut(*source) {
            let component_id = &component_id.unwrap();
            let type_id = type_id.unwrap();

            // call subscribe
            enter_malkovich_world(&mut source, entity, component_id, &type_id, type_registry);

            // merge subscribers just added
            long_live_the_new_flesh(&mut source, component_id, &type_id, type_registry);
        }
    }
}
