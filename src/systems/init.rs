use bevy::{ ecs::world::World, prelude::* };

use crate::{
    systems::subscribe,
    ComputedImmutable,
    EntityHierarchySet,
    LazyEffect,
    RebuildSubscribers,
};

/// FIXME should we actually just trigger everything that is marked instead of faking it?
pub fn init_effects(
    world: &mut World,
    query_effects: &mut QueryState<(Entity, &LazyEffect), With<RebuildSubscribers>>
) {
    // FIXME add support for triggers
    // collapse the query or get world concurrency errors
    let mut sourced_entities = EntityHierarchySet::new();
    let mut triggered_entities = EntityHierarchySet::new();
    for (entity, effect) in query_effects.iter(world) {
        info!("-preparing sources for effect {:?}", entity);
        sourced_entities.insert(entity, effect.sources.clone());
        triggered_entities.insert(entity, effect.triggers.clone());
    }

    world.resource_scope(|world, type_registry: Mut<AppTypeRegistry>| {
        let type_registry = type_registry.read();

        // run the subscribe method on all Effect.sources
        for (entity, sources) in sourced_entities.iter() {
            // loop through the sources
            for source in sources.iter() {
                subscribe(world, entity, source, &type_registry);
            }

            // mark as processed
            world.get_entity_mut(*entity).unwrap().remove::<RebuildSubscribers>();
        }

        // run the subscribe method on all Effect.triggers
        for (entity, triggers) in triggered_entities.iter() {
            // loop through the sources
            for trigger in triggers.iter() {
                subscribe(world, entity, trigger, &type_registry);
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
    let mut sourced_entities = EntityHierarchySet::new();
    for (entity, prop) in query_propagators.iter(world) {
        info!("-preparing sources for memo {:?}", entity);
        sourced_entities.insert(entity, prop.sources.clone());
    }

    world.resource_scope(|world, type_registry: Mut<AppTypeRegistry>| {
        let type_registry = type_registry.read();

        // run the subscribe method on each Propagator.sources, passing the Entity
        for (entity, sources) in sourced_entities.iter() {
            // loop through the sources
            for source in sources.iter() {
                subscribe(world, entity, source, &type_registry);
            }

            // mark as processed
            world.get_entity_mut(*entity).unwrap().remove::<RebuildSubscribers>();
        }
    });
}
