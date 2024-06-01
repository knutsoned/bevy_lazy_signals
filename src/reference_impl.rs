use std::{ any::TypeId, sync::RwLockReadGuard };

use bevy::{ ecs::component::ComponentId, prelude::*, reflect::{ DynamicTuple, TypeRegistry } };

use crate::{ arcane_wizardry::*, framework::* };

/// These are the reference user API systems, patterned after the TC39 proposal.
///
/// Shared reactive context resource, aka global state.
/// This tracks long-running effects across ticks but otherwise should start fresh each cycle.
/// Main purpose is to provide "stack"-like functionality across systems in the processing chain.
#[derive(Resource)]
pub struct LazySignalsResource {
    /// Tracks triggered entities (Signals to send even if their value did not change).
    pub triggered: EntitySet,

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
            changed: empty_set(),
            deferred: empty_set(),
            effects: empty_set(),
            errors: ErrorSet::new(),
        }
    }
}

fn add_subs(subs: &[Entity], triggered: bool, signals: &mut Mut<LazySignalsResource>) {
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
fn process_subs(
    world: &mut World,
    entity: &Entity,
    source: &Entity,
    type_registry: &RwLockReadGuard<TypeRegistry>
) {
    // get the TypeId of each source (Signal or Memo) Immutable component
    let mut component_id: Option<ComponentId> = None;
    let mut type_id: Option<TypeId> = None;

    // get a readonly reference to the source entity
    if let Some(source) = world.get_entity(*source) {
        // get the source Immutable component
        if let Some(immutable_state) = source.get::<ImmutableState>() {
            // ...as a SignalsObservable
            component_id = Some(immutable_state.component_id);
            if let Some(info) = world.components().get_info(component_id.unwrap()) {
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

/// ## Systems
/// These systems are meant to be run in tight sequence, preferably like the plugin demonstrates.
/// Any commands in each system must be applied before proceeding to the next.
///
/// FIXME should we actually just trigger everything that is marked instead of faking it?
pub fn init_effects(
    world: &mut World,
    query_effects: &mut QueryState<(Entity, &LazyEffect), With<RebuildSubscribers>>
) {
    // FIXME add support for triggers
    // collapse the query or get world concurrency errors
    let mut entities = EntityHierarchySet::new();
    for (entity, prop) in query_effects.iter(world) {
        info!("-preparing sources for effect {:?}", entity);
        entities.insert(entity, prop.sources.clone());
    }

    world.resource_scope(|world, type_registry: Mut<AppTypeRegistry>| {
        let type_registry = type_registry.read();

        // run the subscribe method on each Effect.triggers, passing the Entity
        for (entity, sources) in entities.iter() {
            // loop through the sources
            for source in sources.iter() {
                // FIXME should this be done with some kind of unsafe entity cell?
                process_subs(world, entity, source, &type_registry);
            }

            // mark as processed
            world.get_entity_mut(*entity).unwrap().remove::<RebuildSubscribers>();
        }
    });
}

/// FIXME we should actually just compute everything that is marked instead of faking it
pub fn init_memos(
    world: &mut World,
    query_propagators: &mut QueryState<(Entity, &ComputedImmutable), With<RebuildSubscribers>>
) {
    // collapse the query or get world concurrency errors
    let mut entities = EntityHierarchySet::new();
    for (entity, prop) in query_propagators.iter(world) {
        info!("-preparing sources for memo {:?}", entity);
        entities.insert(entity, prop.sources.clone());
    }

    world.resource_scope(|world, type_registry: Mut<AppTypeRegistry>| {
        let type_registry = type_registry.read();

        // run the subscribe method on each Propagator.sources, passing the Entity
        for (entity, sources) in entities.iter() {
            // loop through the sources
            for source in sources.iter() {
                // FIXME should this be done with some kind of unsafe entity cell?
                process_subs(world, entity, source, &type_registry);
            }

            // mark as processed
            world.get_entity_mut(*entity).unwrap().remove::<RebuildSubscribers>();
        }
    });
}

pub fn send_signals(
    world: &mut World,
    query_signals: &mut QueryState<(Entity, &ImmutableState), With<SendSignal>>
) {
    trace!("SIGNALS");

    // Phase One: find all the updated signals and schedule their direct subscribers to run
    world.resource_scope(|world, mut signals: Mut<LazySignalsResource>| {
        // initialize sets
        signals.init();

        trace!("looking for signals");
        let mut count = 0;

        let mut component_id_set = ComponentIdSet::new();
        let mut component_info = ComponentInfoSet::new();

        // build component id -> info map
        for (entity, immutable) in query_signals.iter(world) {
            let component_id = immutable.component_id;
            trace!("-found a signal with component ID {:#?}", component_id);
            component_id_set.insert(entity, component_id);
            if let Some(info) = world.components().get_info(component_id) {
                component_info.insert(component_id, info.clone());
            }
            count += 1;
        }
        trace!("found {} signals to send", count);

        // build reflect types for merge operation on reflected UntypedObservable trait object
        world.resource_scope(|world, type_registry: Mut<AppTypeRegistry>| {
            let type_registry = type_registry.read();

            for (entity, component_id) in component_id_set.iter() {
                // here we need to access the Signal as an UntypedObservable & run the merge method
                let component_id = *component_id;
                let mut signal_to_send = world.entity_mut(*entity);

                // use the type_id from the component info, YOLO
                let info = component_info.get(component_id).unwrap();
                let type_id = info.type_id().unwrap();
                // the type_id matches the concrete type of the Signal's generic Immutable

                // it comes from ComponentInfo which is retrieved from the ECS world

                // the component_id is saved when command to make concrete Immutable runs

                // merge the next data value and return a list of subscribers to the change
                // and whether these subscribers should be triggered too
                let subs = the_abyss_gazes_into_you(
                    &mut signal_to_send,
                    &component_id,
                    &type_id,
                    &type_registry
                );

                let triggered = subs.1;
                let subs = subs.0;
                let entity = *entity;

                // if triggered, add to triggered set
                if triggered {
                    signals.triggered.insert(entity, ());
                } else if !subs.is_empty() {
                    // OR if the merge returns a non-zero length list of subscribers, it changed
                    // (for our purposes, anyway)
                    signals.changed.insert(entity, ());
                }

                add_subs(&subs, triggered, &mut signals);

                // mark as processed
                signal_to_send.remove::<SendSignal>();
            }

            // Phase Two: fire notifications up the subscriber tree

            let mut count = 0;
            // as long as there is a next_running set, move next_running set into the current one
            while signals.merge_running() {
                count += 1;
                info!("Sending signals iteration {}", count);

                // make a local copy of the running set
                let mut running = empty_set();
                for runner in signals.running.indices() {
                    // skip if already in processed set
                    if !signals.processed.contains(runner) {
                        info!("...adding {:#?} to running set", runner);

                        running.insert(runner, ());
                    }
                }

                // get an item from the running set
                for runner in running.indices() {
                    // add the item to the processed set
                    signals.processed.insert(runner, ());

                    // what kind of subscriber is this?
                    if let Some(mut subscriber) = world.get_entity_mut(runner) {
                        if subscriber.contains::<LazyEffect>() {
                            // it is an effect, so schedule the effect by adding DeferredEffect
                            subscriber.insert(DeferredEffect);
                            info!("-scheduled effect {:#?}", runner);
                        }
                        if subscriber.contains::<ComputedImmutable>() {
                            // it is a memo, so mark it for recalculation by adding ComputeMemo
                            subscriber.insert(ComputeMemo);
                            info!("-marked memo {:#?} for computation", runner);

                            let component_id = subscriber
                                .get::<ImmutableState>()
                                .unwrap().component_id;
                            let type_id = subscriber
                                .get::<ComputedImmutable>()
                                .unwrap().lazy_immutable_type;
                            info!(
                                "--got component_id {:?} and type_id {:?}",
                                component_id,
                                type_id
                            );

                            // get a list of subscribers
                            let subs = this_is_bat_country(
                                &mut subscriber,
                                &component_id,
                                &type_id,
                                &type_registry
                            );

                            // computed has its own subscribers, so add those to the next_running set
                            // and mark triggered if appropriate
                            add_subs(&subs, signals.triggered.contains(runner), &mut signals);
                        }
                    }
                }

                // clear the running set at the end of each iteration
                signals.running.clear();
            }
        });
    });
}

pub fn compute_memos(
    world: &mut World,
    query_memos: &mut QueryState<(Entity, &ComputedImmutable), With<ComputeMemo>>
) {
    trace!("MEMOS");
    // need exclusive world access here to update memos immediately

    // run each Propagator function to recalculate memo, adding it and sources to the running set
    // do not run this Propagator if already in the processed set
    // do not add a source if source already in the processed set

    // if a source is marked dirty, add it to the running set

    // main loop: evaluate highest index,
    // evaluate that source as above

    // if all sources are up to date, then recompute

    // *** update the data in the cell

    // add the ComputedImmutable to the processed set

    // add to the changed set if the value actually changed

    // remove the ComputeMemo component

    // merge all next_subscribers sets into subscribers
}

pub fn apply_deferred_effects(
    world: &mut World,
    query_effects: &mut QueryState<(Entity, &LazyEffect), With<DeferredEffect>>
) {
    trace!("EFFECTS");
    let mut effects = empty_set();

    // FIXME add support for triggers
    // collapse the query or get world concurrency errors
    let mut hierarchy = EntityHierarchySet::new();
    for (entity, effect) in query_effects.iter(world) {
        hierarchy.insert(entity, effect.sources.clone());
    }

    // read (mostly)
    world.resource_scope(|world, signals: Mut<LazySignalsResource>| {
        for (entity, sources) in hierarchy.iter() {
            // only run an effect if at least one of its triggers is in the changed set
            // OR if it has been explicitly triggered
            let mut actually_run = false;
            if signals.triggered.contains(*entity) {
                info!("-triggering effect {:#?}", entity);
                actually_run = true;
            } else {
                for source in sources {
                    info!("-checking changed set for trigger {:#?}", source);
                    if signals.changed.contains(*source) {
                        info!("-running effect {:#?} with sources {:#?}", entity, sources);
                        actually_run = true;
                    }
                }
            }
            if actually_run {
                effects.insert(*entity, ());
            } else {
                world.resource_scope(|world, type_registry: Mut<AppTypeRegistry>| {
                    let type_registry = type_registry.read();

                    // make sure if effects are deferred but not run that they still refresh
                    // otherwise they will not be notified next time
                    for source in sources {
                        process_subs(world, entity, source, &type_registry);
                    }
                });
            }

            // remove the DeferredEffect component
            world.entity_mut(*entity).remove::<DeferredEffect>();
        }
    });

    // write
    for entity in effects.indices() {
        if let Some(sources) = hierarchy.get(entity) {
            info!("-found effect with sources {:#?}", sources);

            // add the source component ID to the set (probably could be optimized)
            let mut component_id_set = ComponentIdSet::new();
            let mut component_info = ComponentInfoSet::new();

            // build component id -> info map
            for source in sources.iter() {
                let immutable = world.entity(*source).get::<ImmutableState>().unwrap();
                let component_id = immutable.component_id;
                info!("-found a source with component ID {:#?}", component_id);
                component_id_set.insert(*source, component_id);
                if let Some(info) = world.components().get_info(component_id) {
                    component_info.insert(component_id, info.clone());
                }
            }

            world.resource_scope(|world, type_registry: Mut<AppTypeRegistry>| {
                let type_registry = type_registry.read();
                // prepare the params
                let mut params = DynamicTuple::default();
                for source in sources.iter() {
                    let component_id = component_id_set.get(*source).unwrap();
                    let type_id = component_info.get(*component_id).unwrap().type_id().unwrap();

                    // call the copy_data method via reflection
                    // this will append the source data to the params tuple
                    // FIXME indicate an error if the params don't line up?
                    if let Some(mut source) = world.get_entity_mut(*source) {
                        // insert arcane wizardry here
                        ph_nglui_mglw_nafh_cthulhu_r_lyeh_wgah_nagl_fhtagn(
                            &mut source,
                            &entity,
                            &mut params,
                            component_id,
                            &type_id,
                            &type_registry
                        );
                    }
                }

                // actually run the effect
                world.resource_scope(|world, mut _signals: Mut<LazySignalsResource>| {
                    let world = world.as_unsafe_world_cell();
                    if let Some(handle) = world.get_entity(entity) {
                        // safety (from the docs):
                        // -the UnsafeEntityCell has permission to access the component mutably
                        // -no other references to the component exist at the same time
                        unsafe {
                            let mut effect = handle.get_mut::<LazyEffect>().unwrap();

                            // I think this world must not be used to mutate the effect, not sure
                            (effect.function)(&params, world.world_mut());
                        }
                    }
                });
            });
        }
    }
}
