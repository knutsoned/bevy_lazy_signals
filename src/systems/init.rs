use bevy::{ ecs::world::World, prelude::* };

use crate::{ arcane_wizardry::*, framework::* };

type DerivedParam<'a> = (Entity, Option<&'a ComputedImmutable>, Option<&'a LazyEffect>);

// FIXME should we actually just compute and trigger everything that is marked instead of faking it?
pub fn init_deriveds(
    world: &mut World,
    query_deriveds: &mut QueryState<DerivedParam, With<InitDependencies>>
) {
    let mut relationships = EntityRelationshipSet::new();

    // build the branches of the subscriber trees
    query_deriveds.iter(world).for_each(|(entity, computed, effect)| {
        let mut subs = Vec::<Entity>::new();
        if let Some(computed) = computed {
            subs.append(&mut computed.sources.clone());
        }
        if let Some(effect) = effect {
            subs.append(&mut effect.sources.clone());
            subs.append(&mut effect.triggers.clone());
        }
        relationships.insert(entity, subs);
    });

    // run the subscribe method on all sources and triggers
    world.resource_scope(|world, type_registry: Mut<AppTypeRegistry>| {
        let type_registry = type_registry.read();
        for (entity, subs) in relationships.iter() {
            // loop through the sources
            for source in subs.iter() {
                subscribe(entity, source, &type_registry, world);
            }

            // mark as processed
            world.get_entity_mut(*entity).unwrap().remove::<InitDependencies>();
        }
    });
}
