use bevy::{ ecs::world::World, prelude::* };

use crate::{ arcane_wizardry::*, framework::* };

type DerivedParam<'a> = (Entity, Option<&'a ComputedImmutable>, Option<&'a LazyEffect>);
// remove ValueChanged components
pub fn init_lazy_signals(
    world: &mut World,
    query_deriveds: &mut QueryState<DerivedParam, With<InitDependencies>>,
    query_value_changed: &mut QueryState<Entity, With<ValueChanged>>
) {
    // reset the internal change tracking
    let mut changed = empty_set();
    for entity in query_value_changed.iter_mut(world) {
        changed.insert(entity, ());
    }
    for (entity, _) in changed.iter() {
        world.entity_mut(*entity).remove::<ValueChanged>();
    }

    // build the branches of the subscriber trees
    // FIXME should we actually just compute and trigger everything that is marked instead of faking it?
    let mut relationships = EntityRelationshipSet::new();

    query_deriveds.iter(world).for_each(|(entity, computed, effect)| {
        let mut subs = LazySignalsVec::new();
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
            for source in subs.clone().into_iter() {
                subscribe(entity, &source, &type_registry, world);
            }

            // mark as processed
            world.get_entity_mut(*entity).unwrap().remove::<InitDependencies>();
        }
    });
}
