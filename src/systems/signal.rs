use bevy::{ ecs::world::World, prelude::* };

use crate::{
    arcane_wizardry::{ the_abyss_gazes_into_you, this_is_bat_country },
    empty_set,
    systems::add_subs_to_running,
    ComponentIdSet,
    ComponentInfoSet,
    ComputeMemo,
    ComputedImmutable,
    DeferredEffect,
    ImmutableState,
    LazyEffect,
    LazySignalsResource,
    SendSignal,
};

pub fn send_signals(
    world: &mut World,
    query_signals: &mut QueryState<(Entity, &ImmutableState), With<SendSignal>>
) {
    trace!("SIGNALS");

    // Phase One: find all the updated signals and schedule their direct subscribers to run
    world.resource_scope(|world, mut signals: Mut<LazySignalsResource>| {
        // initialize sets
        signals.init();

        trace!("looking for signals");
        let mut count = 0;

        let mut component_id_set = ComponentIdSet::new();
        let mut component_info = ComponentInfoSet::new();

        // build component id -> info map
        for (entity, immutable) in query_signals.iter(world) {
            let component_id = immutable.component_id;
            trace!("-found a signal with component ID {:#?}", component_id);
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
                let info = component_info.get(component_id).unwrap();
                let type_id = info.type_id().unwrap();
                // the type_id matches the concrete type of the Signal's generic Immutable

                // it comes from ComponentInfo which is retrieved from the ECS world

                // the component_id is saved when command to make concrete Immutable runs

                // merge the next data value and return a list of subscribers to the change
                // and whether these subscribers should be triggered too
                let subs = the_abyss_gazes_into_you(
                    &mut signal_to_send,
                    &component_id,
                    &type_id,
                    &type_registry
                );

                let triggered = subs.1;
                let subs = subs.0;
                let entity = *entity;

                // if the merge returns a non-zero length list of subscribers, it changed
                // (for our purposes, anyway)
                if !triggered && !subs.is_empty() {
                    signals.changed.insert(entity, ());
                }

                // add subscribers to the running set and mark if triggered
                //info!("SUBS for {:#?} are: {:#?}", entity, subs);
                add_subs_to_running(&subs, triggered, &mut signals);

                // mark as processed
                signal_to_send.remove::<SendSignal>();
            }

            // Phase Two: fire notifications up the subscriber tree

            let mut count = 0;
            // as long as there is a next_running set, move next_running set into the current one
            while signals.merge_running() {
                count += 1;
                trace!("Sending signals iteration {}", count);

                // make a local copy of the running set
                let mut running = empty_set();
                for runner in signals.running.indices() {
                    // skip if already in processed set
                    if !signals.processed.contains(runner) {
                        trace!("...adding {:#?} to running set", runner);

                        running.insert(runner, ());
                    }
                }

                // get an item from the running set
                for runner in running.indices() {
                    // add the item to the processed set
                    signals.processed.insert(runner, ());

                    // what kind of subscriber is this?
                    if let Some(mut subscriber) = world.get_entity_mut(runner) {
                        if subscriber.contains::<LazyEffect>() {
                            // it is an effect, so schedule the effect by adding DeferredEffect
                            subscriber.insert(DeferredEffect);
                            trace!("-scheduled effect {:#?}", runner);
                        }
                        if subscriber.contains::<ComputedImmutable>() {
                            // it is a memo, so mark it for recalculation by adding ComputeMemo
                            subscriber.insert(ComputeMemo);
                            trace!("-marked memo {:#?} for computation", runner);

                            let component_id = subscriber
                                .get::<ImmutableState>()
                                .unwrap().component_id;
                            let type_id = subscriber
                                .get::<ComputedImmutable>()
                                .unwrap().lazy_immutable_type;
                            trace!(
                                "--got component_id {:?} and type_id {:?}",
                                component_id,
                                type_id
                            );

                            // get a list of subscribers
                            let subs = this_is_bat_country(
                                &mut subscriber,
                                &component_id,
                                &type_id,
                                &type_registry
                            );

                            // computed has its own subscribers, so add those to the next_running set
                            // and mark triggered if appropriate
                            add_subs_to_running(
                                &subs,
                                signals.triggered.contains(runner),
                                &mut signals
                            );
                        }
                    }
                }

                // clear the running set at the end of each iteration
                signals.running.clear();
            }
        });
    });
}
