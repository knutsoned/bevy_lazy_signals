pub mod computed;
pub mod effect;
pub mod init;
pub mod signal;

use std::{ any::TypeId, sync::RwLockReadGuard };

use bevy::{ ecs::component::ComponentId, prelude::*, reflect::TypeRegistry };

use crate::{ arcane_wizardry::*, framework::*, LazySignalsResource };

/// These are the reference user API systems, patterned after the TC39 proposal.
fn add_subs_to_hierarchy(
    query_effects: &mut QueryState<(Entity, &LazyEffect), With<DeferredEffect>>,
    hierarchy: &mut EntityHierarchySet,
    subs_closure: Box<dyn EffectSubsFn>,
    world: &mut World
) {
    for (entity, effect) in query_effects.iter(world) {
        let subs = hierarchy.get_or_insert_with(entity, || { Vec::new() });
        subs.append(&mut subs_closure(effect));
    }
}

fn add_subs_to_running(subs: &[Entity], triggered: bool, signals: &mut Mut<LazySignalsResource>) {
    // add subscribers to the next running set
    for subscriber in subs.iter() {
        let subscriber = *subscriber;
        signals.next_running.insert(subscriber, ());
        trace!("-added subscriber {:?} to running set", subscriber);

        // if these subs were triggered, they need to be marked triggered too
        if triggered {
            // add each one to the triggered set
            signals.triggered.insert(subscriber, ());
        }
    }
}

/// Convenience function to subscribe an entity to a source.
fn subscribe(
    entity: &Entity,
    source: &Entity,
    type_registry: &RwLockReadGuard<TypeRegistry>,
    world: &mut World
) {
    // get the TypeId of each source (Signal or Memo) Immutable component
    let mut component_id: Option<ComponentId> = None;
    let mut type_id: Option<TypeId> = None;

    trace!("Subscribing {:#?} to {:?}", entity, source);

    // get a readonly reference to the source entity
    if let Some(source) = world.get_entity(*source) {
        trace!("-got source EntityRef");
        // get the source Immutable component
        if let Some(immutable_state) = source.get::<ImmutableState>() {
            trace!("-got ImmutableState");
            // ...as a SignalsObservable
            component_id = Some(immutable_state.component_id);
            if let Some(info) = world.components().get_info(component_id.unwrap()) {
                trace!("-got TypeId");
                type_id = info.type_id();
            }
        }
    }

    // we have a component and a type, now do mutable stuff
    if component_id.is_some() && type_id.is_some() {
        if let Some(mut source) = world.get_entity_mut(*source) {
            let component_id = &component_id.unwrap();
            let type_id = type_id.unwrap();

            run_observable_method(
                &mut source,
                None,
                Some(entity),
                component_id,
                &type_id,
                type_registry,
                Box::new(|observable, _params, target| {
                    observable.subscribe(*target.unwrap());
                    observable.merge_subscribers();
                    None
                })
            );
        }
    }
}

fn subscribe_effect_subs(
    query_effects: &mut QueryState<(Entity, &LazyEffect), With<RebuildSubscribers>>,
    subs_closure: Box<dyn EffectSubsFn>,
    type_registry: &RwLockReadGuard<TypeRegistry>,
    world: &mut World
) {
    let mut hierarchy = EntityHierarchySet::new();

    // run the subscribe method on all Effect.sources
    for (entity, effect) in query_effects.iter(world) {
        hierarchy.insert(entity, subs_closure(effect));
    }

    for (entity, subs) in hierarchy.iter() {
        // loop through the sources
        for source in subs.iter() {
            subscribe(entity, source, type_registry, world);
        }

        // mark as processed
        world.get_entity_mut(*entity).unwrap().remove::<RebuildSubscribers>();
    }
}

fn subscribe_propagator_subs(
    query_propagators: &mut QueryState<(Entity, &ComputedImmutable), With<RebuildSubscribers>>,
    subs_closure: Box<dyn PropagatorSubsFn>,
    type_registry: &RwLockReadGuard<TypeRegistry>,
    world: &mut World
) {
    let mut hierarchy = EntityHierarchySet::new();

    // run the subscribe method on all Effect.sources
    for (entity, effect) in query_propagators.iter(world) {
        hierarchy.insert(entity, subs_closure(effect));
    }

    for (entity, subs) in hierarchy.iter() {
        // loop through the sources
        for source in subs.iter() {
            subscribe(entity, source, type_registry, world);
        }

        // mark as processed
        world.get_entity_mut(*entity).unwrap().remove::<RebuildSubscribers>();
    }
}
