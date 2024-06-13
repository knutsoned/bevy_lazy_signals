use bevy::{ ecs::world::World, prelude::*, reflect::DynamicTuple };

use crate::{ arcane_wizardry::*, framework::* };

pub fn compute_memos(
    world: &mut World,
    query_memos: &mut QueryState<(Entity, &ImmutableState, &ComputedImmutable), With<ComputeMemo>>
) {
    trace!("MEMOS");

    let mut component_id_set = ComponentIdSet::new();
    let mut component_info_set = ComponentInfoSet::new();
    let mut processed = empty_set();
    let mut sources = EntityRelationshipSet::new();
    let mut stack = Vec::<Entity>::new();

    query_memos.iter(world).for_each(|(entity, immutable, computed)| {
        let component_id = immutable.component_id;
        trace!("-found computed {:#?} with component ID {:?}", entity, component_id);
        component_id_set.insert(entity, component_id);
        if let Some(info) = world.components().get_info(component_id) {
            component_info_set.insert(component_id, info.clone());
        }

        sources.insert(entity, computed.sources.clone());

        // doesn't matter what order we evaluate things in since it all has to get resolved
        // the value of each computed memo is deterministic since the data is immutable
        stack.push(entity);
    });

    // main loop: evaluate highest index (pop the stack)
    while let Some(computed) = stack.pop() {
        // do not run this Propagator if already in the processed set
        if processed.contains(computed) {
            continue;
        }

        let sources = sources.get(computed).unwrap();
        let mut dirty_sources = Vec::<Entity>::new();
        for source in sources {
            let source = *source;
            if world.entity(source).contains::<Dirty>() {
                dirty_sources.push(source);
            }
        }

        // if any sources are marked dirty, push them on the stack, after the memo
        if !dirty_sources.is_empty() {
            stack.push(computed);
            stack.append(&mut dirty_sources);
        } else {
            // otherwise, if all sources are up to date, then recompute

            // build component id -> info map (might already have some but be on the safe side)
            for source in sources.iter() {
                let immutable = world.entity(*source).get::<ImmutableState>().unwrap();
                let component_id = immutable.component_id;
                trace!("-found a computed source with component ID {:#?}", component_id);
                component_id_set.insert(*source, component_id);
                if let Some(info) = world.components().get_info(component_id) {
                    component_info_set.insert(component_id, info.clone());
                }
            }

            // remove the ComputeMemo component
            world.entity_mut(computed).remove::<ComputeMemo>();

            world.resource_scope(|world, type_registry: Mut<AppTypeRegistry>| {
                let type_registry = type_registry.read();

                // prepare the args
                let mut args = DynamicTuple::default();
                for source in sources.iter() {
                    let component_id = component_id_set.get(*source).unwrap();
                    let type_id = component_info_set.get(*component_id).unwrap().type_id().unwrap();

                    // call the copy_data method via reflection
                    // this will append the source data to the args tuple
                    // FIXME indicate an error if the args don't line up?
                    if let Some(mut source) = world.get_entity_mut(*source) {
                        // insert arcane wizardry here
                        run_as_observable(
                            &mut source,
                            Some(&mut args),
                            Some(&computed),
                            component_id,
                            &type_id,
                            &type_registry,
                            Box::new(|observable, args, target| {
                                observable.copy_data(*target.unwrap(), args.unwrap());
                                None
                            })
                        );
                    }

                    // make sure computeds refresh so they will be notified next time
                    subscribe(&computed, source, &type_registry, world);
                }

                let mut changed = false;
                let mut clean = false;

                // actually compute the computed
                {
                    let world = world.as_unsafe_world_cell();
                    if let Some(handle) = world.get_entity(computed) {
                        // safety (from the docs):
                        // -the UnsafeEntityCell has permission to access the component mutably
                        // -no other references to the component exist at the same time
                        unsafe {
                            let computed_immutable = handle.get_mut::<ComputedImmutable>().unwrap();

                            // I think this world must not be used to mutate the computed, not sure
                            if
                                computed_immutable.function
                                    .lock()
                                    .unwrap()(&args, &computed, world.world_mut())
                            {
                                // mark changed if the value actually changed
                                changed = true;
                            }
                        }

                        // add the computed entity to the processed set
                        processed.insert(computed, ());

                        // mark the computed not dirty
                        clean = true;
                    }
                }

                if changed || clean {
                    let mut handle = world.entity_mut(computed);

                    if changed {
                        handle.insert(ValueChanged);
                    }

                    if clean {
                        handle.remove::<Dirty>();
                    }
                }
            });
        }
    }
}
