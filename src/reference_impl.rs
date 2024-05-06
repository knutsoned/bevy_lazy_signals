use std::any::TypeId;

use bevy::{ ecs::component::ComponentId, prelude::*, reflect::DynamicTuple };

use crate::{ arcane_wizardry::*, signals::*, SignalsResource };

/// This is the reference user API, patterned after the TC39 proposal.
///
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

        // run the subscribe method on each Propagator.sources, passing the Entity
        for (entity, sources) in entities.iter() {
            // loop through the sources
            for source in sources.iter() {
                // get the TypeId of each source (Signal or Memo) Immutable component
                let mut component_id: Option<ComponentId> = None;
                let mut type_id: Option<TypeId> = None;

                // get a readonly reference to the source entity
                if let Some(source) = world.get_entity(*source) {
                    // get the source Immutable component
                    if let Some(immutable) = source.get::<ImmutableComponentId>() {
                        // ...as an UntypedObservable
                        component_id = Some(immutable.component_id);
                        if let Some(info) = world.components().get_info(component_id.unwrap()) {
                            type_id = info.type_id();
                        }
                    }
                }

                // we have a component and a type, now do mutable stuff
                if component_id.is_some() && type_id.is_some() {
                    if let Some(mut source) = world.get_entity_mut(*source) {
                        let type_id = type_id.unwrap();

                        // call subscribe
                        enter_malkovich_world(
                            &mut source,
                            entity,
                            &component_id.unwrap(),
                            &type_id,
                            &type_registry
                        );

                        // get the source Immutable component as an ECS change detection handle
                        let mut mut_untyped = source.get_mut_by_id(component_id.unwrap()).unwrap();

                        // merge the new subscriber into the main set
                        let ptr_mut = mut_untyped.as_mut();

                        // insert arcane wizardry here
                        let reflect_from_ptr = make_reflect_from_ptr(type_id, &type_registry);
                        long_live_the_new_flesh(ptr_mut, &reflect_from_ptr, &type_registry);
                    }
                }
            }

            world.get_entity_mut(*entity).unwrap().remove::<RebuildSubscribers>();
        }
    });
}

pub fn init_propagators(
    world: &mut World,
    query_propagators: &mut QueryState<(Entity, &Propagator), With<RebuildSubscribers>>
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
                // get the TypeId of each source (Signal or Memo) Immutable component
                let mut component_id: Option<ComponentId> = None;
                let mut type_id: Option<TypeId> = None;

                // get a readonly reference to the source entity
                if let Some(source) = world.get_entity(*source) {
                    // get the source Immutable component
                    if let Some(immutable) = source.get::<ImmutableComponentId>() {
                        // ...as an UntypedObservable
                        component_id = Some(immutable.component_id);
                        if let Some(info) = world.components().get_info(component_id.unwrap()) {
                            type_id = info.type_id();
                        }
                    }
                }

                info!("component_id: {:?}, type_id: {:?}", component_id, type_id);
                // we have a component and a type, now do mutable stuff
                if component_id.is_some() && type_id.is_some() {
                    if let Some(mut source) = world.get_entity_mut(*source) {
                        let type_id = type_id.unwrap();

                        // call subscribe
                        enter_malkovich_world(
                            &mut source,
                            entity,
                            &component_id.unwrap(),
                            &type_id,
                            &type_registry
                        );

                        // get the source Immutable component as an ECS change detection handle
                        let mut mut_untyped = source.get_mut_by_id(component_id.unwrap()).unwrap();

                        // merge the new subscriber into the main set
                        let ptr_mut = mut_untyped.as_mut();

                        // insert arcane wizardry here
                        let reflect_from_ptr = make_reflect_from_ptr(type_id, &type_registry);
                        long_live_the_new_flesh(ptr_mut, &reflect_from_ptr, &type_registry);
                    }
                }
            }

            world.get_entity_mut(*entity).unwrap().remove::<RebuildSubscribers>();
        }
    });
}

pub fn send_signals(
    world: &mut World,
    query_signals: &mut QueryState<(Entity, &ImmutableComponentId), With<SendSignal>>
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
                if let Some(info) = component_info.get(component_id) {
                    if let Some(type_id) = info.type_id() {
                        // the type_id matches the concrete type of the Signal's generic Immutable

                        // it comes from ComponentInfo which is retrieved from the ECS world

                        // the component_id is saved when command to make concrete Immutable runs

                        // get like... an ECS change detection handle for the component in question
                        let mut mut_untyped = signal_to_send.get_mut_by_id(component_id).unwrap();

                        // ...and convert that into a pointer
                        let ptr_mut = mut_untyped.as_mut();

                        // insert arcane wizardry here
                        let reflect_from_ptr = make_reflect_from_ptr(type_id, &type_registry);

                        // merge the next data value and return a list of subscribers to the change
                        let subs = the_abyss_gazes_into_you(
                            ptr_mut,
                            &reflect_from_ptr,
                            &type_registry
                        );

                        // add subscribers to the next running set
                        for subscriber in subs.into_iter() {
                            signals.next_running.insert(subscriber, ());
                            info!("-added subscriber {:?} to running set", subscriber);
                        }

                        // if the merge returns a non-zero length list of subscribers, it changed
                        signals.changed.insert(*entity, ());
                    }
                }

                // remove the Signal component
                signal_to_send.remove::<SendSignal>();
            }

            // Phase Two: fire notifications up the subscriber tree

            // as long as there is a next_running set, move next_running set into the current one
            while signals.merge_running() {
                // make a copy of the running set
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
                        // if the entity has a Propagator
                        if subscriber.contains::<Propagator>() {
                            // it is a memo, so mark it for recalculation by adding ComputeMemo
                            subscriber.insert(ComputeMemo);
                            info!("-marked memo for computation");

                            // FIXME item has its own subscribers, so add those to the next_running set
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
    query_memos: &mut QueryState<(Entity, &Propagator), With<ComputeMemo>>
) {
    trace!("MEMOS");
    // need exclusive world access here to update memos immediately and need to write to resource
    world.resource_scope(
        |world, mut signals: Mut<SignalsResource>| {
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
                info!("-checking changed set");
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
            info!("-found effect with sources {:?}", sources);

            // add the source component ID to the set (probably could be optimized)
            let mut component_id_set = ComponentIdSet::new();
            let mut component_info = ComponentInfoSet::new();

            // build component id -> info map
            for source in sources.iter() {
                let immutable = world.entity(*source).get::<ImmutableComponentId>().unwrap();
                let component_id = immutable.component_id;
                info!("-found a trigger with component ID {:?}", component_id);
                component_id_set.insert(entity, component_id);
                if let Some(info) = world.components().get_info(component_id) {
                    component_info.insert(component_id, info.clone());
                }
            }

            // actually run the effect
            // let mut params = DynamicTuple::default();
            for (_, component_id) in component_id_set.iter() {
                // should be able to call the value method via reflection
                world.resource_scope(|world, type_registry: Mut<AppTypeRegistry>| {
                    let type_registry = type_registry.read();

                    info!("component_id: {:?}", component_id);
                    let type_id = component_info.get(*component_id).unwrap().type_id();

                    // this may be a little trickier since we need to do generics
                    //let registration = type_registry.get(component_id.type_id()).unwrap();
                    info!("type_id: {:?}", type_id);

                    // looks like we need the type_id to come from component_info
                    // since the one from component_id is probably the ComponentIfd itself
                    // and not the actual Component
                    let registration = type_registry.get(type_id.unwrap()).unwrap();
                    info!("Registration for {} exists", registration.type_info().type_path());

                    /* TODO
                    // we can get the proper type registration for the Immutable<T> component
                    // but without knowing T we can't get the right reflect type such as
                    // ReflectLazyImmutable<bool>
                    let mut binding = world.entity_mut(source);

                    // need to call the value method and add the result to the params
                    let mut mut_untyped = binding.get_mut_by_id(*component_id).unwrap();

                    // ...and convert that into a pointer
                    let ptr_mut = mut_untyped.as_mut();

                    // insert arcane wizardry here
                    let reflect_from_ptr = make_reflect_from_ptr(
                        registration.type_id(),
                        &type_registry
                    );

                    // get the data and put it in a dyhnamic tuple
                    params.insert(
                        ph_nglui_mglw_nafh_cthulhu_r_lyeh_wgah_nagl_fhtagn(
                            ptr_mut,
                            &reflect_from_ptr,
                            &type_registry
                        )
                    );
                    */
                });
            }

            // then call the EffectFn with the gathered params
        }
    }
}
