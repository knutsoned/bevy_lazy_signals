use bevy::{ ecs::world::World, prelude::*, reflect::DynamicTuple };

use crate::{
    arcane_wizardry::run_as_observable,
    empty_set,
    framework::*,
    systems::subscribe,
    ComponentIdSet,
    ComponentInfoSet,
    EntityRelationshipSet,
    LazySignalsResource,
};

fn add_deps_to_relationship(
    query_effects: &mut QueryState<(Entity, &LazyEffect), With<DeferredEffect>>,
    relationship: &mut EntityRelationshipSet,
    subs_closure: Box<dyn EffectSubsFn>,
    world: &mut World
) {
    query_effects.iter(world).for_each(|(entity, effect)| {
        let deps = relationship.get_or_insert_with(entity, || { Vec::new() });
        deps.append(&mut subs_closure(effect));
    });
}

pub fn apply_deferred_effects(
    world: &mut World,
    query_effects: &mut QueryState<(Entity, &LazyEffect), With<DeferredEffect>>
) {
    trace!("EFFECTS");
    // collapse the query or get world concurrency errors
    let mut relationship = EntityRelationshipSet::new();
    add_deps_to_relationship(
        query_effects,
        &mut relationship,
        Box::new(|x: &LazyEffect| x.sources.clone()),
        world
    );
    add_deps_to_relationship(
        query_effects,
        &mut relationship,
        Box::new(|x: &LazyEffect| x.triggers.clone()),
        world
    );

    let mut effects = empty_set();

    trace!("Processing effects {:#?}", relationship);

    // read, mostly
    world.resource_scope(|world, signals: Mut<LazySignalsResource>| {
        for (effect, sources) in relationship.iter() {
            let effect = *effect;
            trace!("Processing effect {:?}", effect);

            // only run an effect if at least one of its sources is in the changed set
            // OR it has been explicitly triggered
            let mut actually_run = false;
            if signals.triggered.contains(effect) {
                trace!("-triggering effect {:#?}", effect);
                actually_run = true;
            } else {
                for source in sources {
                    trace!("-checking changed set for source {:#?}", source);
                    if signals.changed.contains(*source) {
                        trace!("-running effect {:#?} with sources {:?}", effect, sources);
                        actually_run = true;
                    }
                }
            }
            if actually_run {
                effects.insert(effect, ());
            }

            // remove the DeferredEffect component
            world.entity_mut(effect).remove::<DeferredEffect>();

            // make sure if effects are deferred but not run that they still refresh
            // otherwise they will not be notified next time
            world.resource_scope(|world, type_registry: Mut<AppTypeRegistry>| {
                let type_registry = type_registry.read();
                for source in sources {
                    subscribe(&effect, source, &type_registry, world);
                }
            });
        }
    });

    // write
    for effect in effects.indices() {
        // FIXME this is probably skipping trigger-only effects
        let sources = relationship.get(effect).map_or(Vec::<Entity>::new(), |s| s.to_vec());
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
            // prepare the params
            let mut params = DynamicTuple::default();
            for source in sources.iter() {
                let component_id = component_id_set.get(*source).unwrap();
                let type_id = component_info_set.get(*component_id).unwrap().type_id().unwrap();

                // call the copy_data method via reflection
                // this will append the source data to the params tuple
                // FIXME indicate an error if the params don't line up?
                if let Some(mut source) = world.get_entity_mut(*source) {
                    // insert arcane wizardry here
                    run_as_observable(
                        &mut source,
                        Some(&mut params),
                        Some(&effect),
                        component_id,
                        &type_id,
                        &type_registry,
                        Box::new(|observable, params, target| {
                            observable.copy_data(*target.unwrap(), params.unwrap());
                            None
                        })
                    );
                }
            }

            // actually run the effect
            world.resource_scope(|world, mut _signals: Mut<LazySignalsResource>| {
                let world = world.as_unsafe_world_cell();
                if let Some(handle) = world.get_entity(effect) {
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
