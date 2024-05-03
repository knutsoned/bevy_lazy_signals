use std::any::TypeId;

use bevy::{ ecs::{ component::{ ComponentId, ComponentInfo }, storage::SparseSet }, prelude::* };

use crate::{
    arcane_wizardry::*,
    ComputeMemo,
    DeferredEffect,
    ImmutableComponentId,
    Propagator,
    RebuildSubscribers,
    SendSignal,
    SignalsResource,
};

/// Set of unique Entities
pub type EntitySourcesSet = SparseSet<Entity, Vec<Entity>>;

/// Set of Entity to ComponentId
pub type ComponentIdSet = SparseSet<Entity, ComponentId>;

/// Set of ComponentId to ComponentInfo
pub type ComponentInfoSet = SparseSet<ComponentId, ComponentInfo>;

/// This is the reference user API, patterned after the TC39 proposal.
///
/// ## Systems
/// These systems are meant to be run in tight sequence, preferably like the plugin demonstrates.
/// Any commands in each system must be applied before proceeding to the next.

pub fn init_subscribers(
    world: &mut World,
    query_propagators: &mut QueryState<(Entity, &Propagator), With<RebuildSubscribers>>
) {
    // collapse the query or get world concurrency errors
    let mut entities = EntitySourcesSet::new();
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

                // now do mutable stuff
                if component_id.is_some() && type_id.is_some() {
                    if let Some(mut source) = world.get_entity_mut(*source) {
                        // get the source Immutable component as an ECS change detection handle
                        let mut mut_untyped = source.get_mut_by_id(component_id.unwrap()).unwrap();

                        // ...and convert that into a pointer
                        let ptr_mut = mut_untyped.as_mut();

                        // insert arcane wizardry here
                        let reflect_from_ptr = make_reflect_from_ptr(
                            type_id.unwrap(),
                            &type_registry
                        );
                        enter_malkovich_world(*entity, ptr_mut, &reflect_from_ptr, &type_registry);

                        let ptr_mut = mut_untyped.as_mut();
                        long_live_the_new_flesh(ptr_mut, &reflect_from_ptr, &type_registry);
                    }
                }
            }

            let mut target = world.get_entity_mut(*entity).unwrap();
            target.remove::<RebuildSubscribers>();
        }
    });
}

pub fn send_signals(
    world: &mut World,
    query_signals: &mut QueryState<(Entity, &ImmutableComponentId), With<SendSignal>>
) {
    trace!("SIGNALS");

    // Phase One:
    world.resource_scope(|world, mut signals: Mut<SignalsResource>| {
        // initialize sets
        signals.init();

        let mut count = 0;
        let mut component_id_set = ComponentIdSet::new();
        let mut component_info = ComponentInfoSet::new();

        trace!("looking for signals");
        // build component id -> info map
        for (entity, immutable) in query_signals.iter(world) {
            let component_id = immutable.component_id;
            trace!("found a signal for component ID {:?}", component_id);
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
                        // the type_id matches the concrete type of the Signal's generic Immutable component

                        // it comes from ComponentInfo which is retrieved from the ECS World Components

                        // the component_id is recorded when the command to make the concrete Immutable runs

                        // get what is basically an ECS change detection handle for the component in question
                        let mut mut_untyped = signal_to_send.get_mut_by_id(component_id).unwrap();

                        // ...and convert that into a pointer
                        let ptr_mut = mut_untyped.as_mut();

                        // insert arcane wizardry here
                        let reflect_from_ptr = make_reflect_from_ptr(type_id, &type_registry);
                        let subs = the_abyss_gazes_into_you(
                            ptr_mut,
                            &reflect_from_ptr,
                            &type_registry
                        );

                        // add subscribers to the running set
                        for subscriber in subs.into_iter() {
                            signals.running.insert(subscriber, ());
                            info!("-added subscriber {:?} into running set", subscriber);
                        }
                    }
                }

                // remove the Signal component
                signal_to_send.remove::<SendSignal>();
            }

            // Phase Two:

            // iterate through a copy of the running set

            // remove an item from the running set

            // skip if already in handled set

            // add the item to the handled set

            // a) item is an effect, so schedule the effect by adding a DeferredEffect component

            // b1) item is a memo, so mark it for recalculation by adding a ComputeMemo component

            // b2) item has its own subscribers, so add those to a new running set

            // loop through the running set until it is empty, then loop through the new running set, and so on
        });
    });
}

pub fn calculate_memos(world: &mut World, query_memos: &mut QueryState<Entity, With<ComputeMemo>>) {
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
    query_effects: Query<Entity, With<DeferredEffect>>,
    mut signals: ResMut<SignalsResource>,
    mut commands: Commands
) {
    trace!("EFFECTS");
    // only run an effect if one of its triggers is in the changed set

    // *** spawn a thread for each effect

    // remove the Effect component
}
