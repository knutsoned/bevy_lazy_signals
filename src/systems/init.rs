use bevy::{ ecs::world::World, prelude::* };

use crate::{ ComputedImmutable, LazyEffect, RebuildSubscribers };

use super::{ subscribe_effect_subs, subscribe_propagator_subs };

// FIXME should we actually just trigger everything that is marked instead of faking it?
pub fn init_effects(
    world: &mut World,
    query_effects: &mut QueryState<(Entity, &LazyEffect), With<RebuildSubscribers>>
) {
    world.resource_scope(|world, type_registry: Mut<AppTypeRegistry>| {
        let type_registry = type_registry.read();

        subscribe_effect_subs(
            query_effects,
            Box::new(|x: &LazyEffect| { x.sources.clone() }),
            &type_registry,
            world
        );

        subscribe_effect_subs(
            query_effects,
            Box::new(|x: &LazyEffect| { x.triggers.clone() }),
            &type_registry,
            world
        );
    });
}

// FIXME should we actually just compute everything that is marked instead of faking it?
pub fn init_memos(
    world: &mut World,
    query_propagators: &mut QueryState<(Entity, &ComputedImmutable), With<RebuildSubscribers>>
) {
    world.resource_scope(|world, type_registry: Mut<AppTypeRegistry>| {
        let type_registry = type_registry.read();

        subscribe_propagator_subs(
            query_propagators,
            Box::new(|x: &ComputedImmutable| { x.sources.clone() }),
            &type_registry,
            world
        );
    });
}
