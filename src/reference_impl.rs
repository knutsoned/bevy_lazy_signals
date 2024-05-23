use std::{ any::TypeId, sync::RwLockReadGuard };

use bevy::{ ecs::component::ComponentId, prelude::*, reflect::{ DynamicTuple, TypeRegistry } };

use crate::{ arcane_wizardry::*, signals::*, SignalsResource };

/// This is the reference user API, patterned after the TC39 proposal.
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
pub fn init_effects(
    world: &mut World,
    query_effects: &mut QueryState<(Entity, &Effect), With<RebuildSubscribers>>
) {
    // collapse the query or get world concurrency errors
    let mut entities = EntityHierarchySet::new();
    for (entity, prop) in query_effects.iter(world) {
        info!("-preparing sources for {:?}", entity);
        entities.insert(entity, prop.triggers.clone());
    }

    world.resource_scope(|world, type_registry: Mut<AppTypeRegistry>| {
        let type_registry = type_registry.read();

        // run the subscribe method on each Effect.triggers, passing the Entity
        for (entity, triggers) in entities.iter() {
            // loop through the sources
            for source in triggers.iter() {
                // FIXME should this be done with some kind of unsafe entity cell?
                process_subs(world, entity, source, &type_registry);
            }

            // mark as processed
            world.get_entity_mut(*entity).unwrap().remove::<RebuildSubscribers>();
        }
    });
}

pub fn init_memos(
    world: &mut World,
    query_propagators: &mut QueryState<(Entity, &Memo), With<RebuildSubscribers>>
) {
    // collapse the query or get world concurrency errors
    let mut entities = EntityHierarchySet::new();
    for (entity, prop) in query_propagators.iter(world) {
        info!("-preparing sources for {:?}", entity);
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
    world.resource_scope(|world, mut signals: Mut<SignalsResource>| {
        // initialize sets
        signals.init();

        trace!("looking for signals");
        let mut count = 0;

        let mut component_id_set = ComponentIdSet::new();
        let mut component_info = ComponentInfoSet::new();

        // build component id -> info map
        for (entity, immutable) in query_signals.iter(world) {
            let component_id = immutable.component_id;
            trace!("-found a signal with component ID {:?}", component_id);
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

                // if the merge returns a non-zero length list of subscribers, it changed
                // (for our purposes, anyway)
                if !triggered && !subs.is_empty() {
                    signals.changed.insert(*entity, ());
                }

                // add subscribers to the next running set
                for subscriber in subs.into_iter() {
                    signals.next_running.insert(subscriber, ());
                    info!("-added subscriber {:?} to running set", subscriber);

                    // if these subs were triggered, they need to be marked triggered too
                    if triggered {
                        // add each one to the triggered set
                        signals.triggered.insert(subscriber, ());
                    }
                }

                // mark as processed
                signal_to_send.remove::<SendSignal>();
            }

            // Phase Two: fire notifications up the subscriber tree

            // as long as there is a next_running set, move next_running set into the current one
            while signals.merge_running() {
                // make a local copy of the running set
                let mut running = empty_set();
                for runner in signals.running.indices() {
                    info!("we've got a runner: {:?}", runner);

                    // skip if already in processed set
                    if !signals.processed.contains(runner) {
                        info!("...adding to running set");

                        running.insert(runner, ());
                    }
                }

                // get an item from the running set
                for runner in running.indices() {
                    // add the item to the processed set
                    signals.processed.insert(runner, ());

                    // what kind of subscriber is this?
                    if let Some(mut subscriber) = world.get_entity_mut(runner) {
                        if subscriber.contains::<Effect>() {
                            // it is an effect, so schedule the effect by adding DeferredEffect
                            subscriber.insert(DeferredEffect);
                            info!("-scheduled effect");
                        }
                        if subscriber.contains::<Memo>() {
                            // it is a memo, so mark it for recalculation by adding ComputeMemo
                            subscriber.insert(ComputeMemo);
                            info!("-marked memo for computation");

                            // FIXME computed has its own subscribers, so add those to the next_running set
                            // and mark triggered if appropriate
                        }
                    }
                }

                // clear the running set at the end of each iteration
                signals.running.clear();
            }
        });
    });
}

pub fn calculate_memos(
    world: &mut World,
    _query_memos: &mut QueryState<(Entity, &Memo), With<ComputeMemo>>
) {
    trace!("MEMOS");
    // need exclusive world access here to update memos immediately and need to write to resource
    world.resource_scope(
        |_world, mut _signals: Mut<SignalsResource>| {
            // run each Memo function to recalculate memo, adding sources to the running set

            // *** update the data in the cell

            // add the Memo to the processed set

            // add to the changed set if the value actually changed

            // remove the Memo component

            // merge all next_subscribers sets into subscribers

        }
    );
}

pub fn apply_deferred_effects(
    world: &mut World,
    query_effects: &mut QueryState<(Entity, &Effect), With<DeferredEffect>>
) {
    trace!("EFFECTS");
    let mut effects = empty_set();

    // collapse the query or get world concurrency errors
    let mut hierarchy = EntityHierarchySet::new();
    for (entity, effect) in query_effects.iter(world) {
        hierarchy.insert(entity, effect.triggers.clone());
    }

    // read
    world.resource_scope(|world, signals: Mut<SignalsResource>| {
        for (entity, triggers) in hierarchy.iter() {
            // only run an effect if at least one of its triggers is in the changed set
            for source in triggers {
                info!("-checking changed set for trigger {:?}", source);
                if signals.changed.contains(*source) {
                    info!("-running effect {:?} with triggers {:?}", entity, triggers);
                    effects.insert(*entity, ());
                }
            }

            // remove the DeferredEffect component
            world.entity_mut(*entity).remove::<DeferredEffect>();
        }
    });

    // write
    for entity in effects.indices() {
        if let Some(sources) = hierarchy.get(entity) {
            info!("-found effect with triggers {:?}", sources);

            // add the source component ID to the set (probably could be optimized)
            let mut component_id_set = ComponentIdSet::new();
            let mut component_info = ComponentInfoSet::new();

            // build component id -> info map
            for source in sources.iter() {
                let immutable = world.entity(*source).get::<ImmutableState>().unwrap();
                let component_id = immutable.component_id;
                info!("-found a trigger with component ID {:?}", component_id);
                component_id_set.insert(*source, component_id);
                if let Some(info) = world.components().get_info(component_id) {
                    component_info.insert(component_id, info.clone());
                }
            }

            world.resource_scope(|world, type_registry: Mut<AppTypeRegistry>| {
                let type_registry = type_registry.read();
                // prepare the params
                let mut params = DynamicTuple::default();
                for (source, component_id) in component_id_set.iter() {
                    // should be able to call the value method via reflection
                    let type_id = component_info.get(*component_id).unwrap().type_id().unwrap();

                    // FIXME throw an error if the params don't line up
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
                world.resource_scope(|world, mut _signals: Mut<SignalsResource>| {
                    if let Some(mut handle) = world.get_entity_mut(entity) {
                        let effect = handle.get_mut::<Effect>().unwrap();
                        (effect.function)(&params);
                    }
                });
            });
        }
    }
}
