use bevy::{ ecs::world::World, prelude::* };

use crate::{ arcane_wizardry::*, framework::* };

fn process_subs(relationships: &EntityRelationshipSet, world: &mut World) {
    world.resource_scope(|world, type_registry: Mut<AppTypeRegistry>| {
        let type_registry = type_registry.read();
        for (entity, subs) in relationships.iter() {
            // loop through the sources
            for source in subs.iter() {
                subscribe(entity, source, &type_registry, world);
            }

            // mark as processed
            world.get_entity_mut(*entity).unwrap().remove::<RebuildSubscribers>();
        }
    });
}

// FIXME should we actually just trigger everything that is marked instead of faking it?
pub fn init_effects(
    world: &mut World,
    query_effects: &mut QueryState<(Entity, &LazyEffect), With<RebuildSubscribers>>
) {
    let mut relationships = EntityRelationshipSet::new();

    // run the subscribe method on all LazyEffect.sources and .triggers
    query_effects.iter(world).for_each(|(entity, effect)| {
        let mut subs = Vec::<Entity>::new();
        subs.append(&mut effect.sources.clone());
        subs.append(&mut effect.triggers.clone());
        relationships.insert(entity, subs);
    });

    process_subs(&relationships, world)
}

// FIXME should we actually just compute everything that is marked instead of faking it?
pub fn init_computeds(
    world: &mut World,
    query_computeds: &mut QueryState<(Entity, &ComputedImmutable), With<RebuildSubscribers>>
) {
    let mut relationships = EntityRelationshipSet::new();

    // run the subscribe method on all ComputedImmutable.sources
    query_computeds.iter(world).for_each(|(entity, computed)| {
        let mut subs = Vec::<Entity>::new();
        subs.append(&mut computed.sources.clone());
        relationships.insert(entity, subs);
    });

    process_subs(&relationships, world)
}
