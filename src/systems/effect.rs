use bevy::{
    ecs::system::CommandQueue,
    prelude::*,
    reflect::DynamicTuple,
    tasks::{ block_on, futures_lite::future, Task },
};

use crate::{ arcane_wizardry::*, framework::* };

type DeferredEffectsParam = (With<DeferredEffect>, Without<RunningTask>);

// get all the currently running tasks
pub fn check_tasks(mut running_tasks: Query<(Entity, &mut RunningTask)>, mut commands: Commands) {
    for (entity, mut running) in &mut running_tasks {
        if let Some(mut commands_queue) = block_on(future::poll_once(&mut running.task)) {
            // append the returned command queue to have it execute later
            commands.append(&mut commands_queue);

            if let Some(mut entity) = commands.get_entity(entity) {
                entity.remove::<RunningTask>();
            }
        }
    }
}

// run all the effects what need running
pub fn apply_deferred_effects(
    world: &mut World,
    query_changed: &mut QueryState<(Entity,), With<ValueChanged>>,
    query_effects: &mut QueryState<(Entity, &LazyEffect, Option<&Triggered>), DeferredEffectsParam>
) {
    trace!("EFFECTS");

    // build a set of changed Computeds and Signals
    let mut changed = empty_set();
    query_changed.iter(world).for_each(|(entity,)| {
        changed.insert(entity, ());
    });

    // store newly created Tasks here
    let mut new_tasks = Vec::<(Entity, Task<CommandQueue>)>::new();

    // collapse the query or get world concurrency errors
    let mut relationships = EntityRelationshipSet::new();
    let mut triggered = empty_set();
    query_effects.iter(world).for_each(|(entity, effect, triggered_effect)| {
        // only add the effect if it isn't already running
        let mut deps = Vec::<Entity>::new();
        deps.append(&mut effect.sources.clone());
        deps.append(&mut effect.triggers.clone());
        relationships.insert(entity, deps);
        if triggered_effect.is_some() {
            triggered.insert(entity, ());
        }
    });

    let mut effects = empty_set();

    trace!("Processing effects {:#?}", relationships);

    // read, mostly
    for (effect, sources) in relationships.iter() {
        let effect = *effect;
        trace!("Processing effect {:?}", effect);

        // only run an effect if at least one of its sources is in the changed set
        // OR it has been explicitly triggered
        let mut actually_run = false;
        if triggered.contains(effect) {
            trace!("-triggering effect {:#?}", effect);
            actually_run = true;
        } else {
            for source in sources {
                trace!("-checking changed set for source {:#?}", source);
                if changed.contains(*source) {
                    trace!("-running effect {:#?} with sources {:?}", effect, sources);
                    actually_run = true;
                }
            }
        }

        let mut entity = world.entity_mut(effect);
        if actually_run {
            effects.insert(effect, ());

            // remove TriggeredEffect so we don't run this again next frame
            entity.remove::<Triggered>();
        }

        // remove the DeferredEffect component
        entity.remove::<DeferredEffect>();

        // make sure if effects are deferred but not run that they still refresh
        // otherwise they will not be notified next time
        world.resource_scope(|world, type_registry: Mut<AppTypeRegistry>| {
            let type_registry = type_registry.read();
            for source in sources {
                subscribe(&effect, source, &type_registry, world);
            }
        });
    }

    // write
    for effect in effects.indices() {
        let sources = relationships.get(effect).map_or(Vec::<Entity>::new(), |s| s.to_vec());
        trace!("-found effect with sources {:#?}", sources);

        // add the source component ID to the set (probably could be optimized)
        let mut component_id_set = ComponentIdSet::new();
        let mut component_info_set = ComponentInfoSet::new();

        // build component id -> info map
        for source in sources.iter() {
            let immutable = world.entity(*source).get::<ImmutableState>().unwrap();
            let component_id = immutable.component_id;
            trace!("-found an effect source with component ID {:#?}", component_id);
            component_id_set.insert(*source, component_id);
            if let Some(info) = world.components().get_info(component_id) {
                component_info_set.insert(component_id, info.clone());
            }
        }

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
                        Some(&effect),
                        component_id,
                        &type_id,
                        &type_registry,
                        Box::new(|observable, args, target| {
                            observable.copy_data(*target.unwrap(), args.unwrap());
                            None
                        })
                    );
                }
            }

            // actually run the effect
            let mut new_task = false;

            // drop the UnsafeWorldCell after this block so we can access the real world again
            {
                let world = world.as_unsafe_world_cell();
                if let Some(handle) = world.get_entity(effect) {
                    // safety (from the docs):
                    // -the UnsafeEntityCell has permission to access the component mutably
                    // -no other references to the component exist at the same time
                    unsafe {
                        let lazy_effect = handle.get::<LazyEffect>().unwrap();
                        let function = &lazy_effect.function;
                        match function {
                            EffectContext::Short(effect) => {
                                // I think this world must not be used to mutate the effect, not sure
                                effect.lock().unwrap()(&args, world.world_mut());
                            }
                            EffectContext::Long(_) => {
                                trace!("Running task {:?}", effect);
                                new_task = true;
                            }
                        }
                    }
                }

                // run and mark the new task
                if new_task {
                    let handle = world.get_entity(effect).unwrap();
                    unsafe {
                        let lazy_effect = handle.get::<LazyEffect>().unwrap();
                        let function = &lazy_effect.function;
                        if let EffectContext::Long(function) = function {
                            let task = function.lock().unwrap()(&args);
                            new_tasks.push((effect, task));
                        }
                    }
                }
            }
        });
    }

    // mark the new tasks as running
    for task in new_tasks.drain(..) {
        world.entity_mut(task.0).insert(RunningTask { task: task.1 });
    }
}
