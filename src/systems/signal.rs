use bevy::{ ecs::world::World, prelude::* };

use crate::{ arcane_wizardry::*, framework::*, LazySignalsResource };

fn add_subs_to_running(
    subs: &[Entity],
    triggered: bool,
    next_running: &mut EntitySet,
    signals: &mut Mut<LazySignalsResource>
) {
    // add subscribers to the next running set
    for subscriber in subs.iter() {
        let subscriber = *subscriber;
        signals.dirty.insert(subscriber, ());
        next_running.insert(subscriber, ());
        trace!("-added subscriber {:?} to running set", subscriber);
        if triggered {
            trace!("Triggering {:?}", subscriber);
            signals.triggered.insert(subscriber, ());
        }
    }
}

fn merge_running(running: &mut EntitySet, next_running: &mut EntitySet) -> bool {
    // if there is a next_running set, move it into the running set and empty it
    if next_running.is_empty() {
        false
    } else {
        for index in next_running.indices() {
            running.insert(index, ());
        }
        next_running.clear();
        true
    }
}

pub fn send_signals(
    world: &mut World,
    query_signals: &mut QueryState<(Entity, &ImmutableState), With<SendSignal>>
) {
    trace!("SIGNALS");

    let mut next_running = empty_set();
    let mut processed = empty_set();
    let mut running = empty_set();

    // Phase One: find all the updated signals and schedule their direct subscribers to run
    world.resource_scope(|world, mut signals: Mut<LazySignalsResource>| {
        // initialize sets
        signals.init();

        trace!("looking for signals");
        let mut count = 0;

        let mut component_id_set = ComponentIdSet::new();
        let mut component_info_set = ComponentInfoSet::new();

        // build component id -> info map
        query_signals.iter(world).for_each(|(entity, immutable)| {
            let component_id = immutable.component_id;
            trace!("-found a signal with component ID {:#?}", component_id);
            component_id_set.insert(entity, component_id);
            if let Some(info) = world.components().get_info(component_id) {
                component_info_set.insert(component_id, info.clone());
            }
            count += 1;
        });
        trace!("found {} signals to send", count);

        // build reflect types for merge operation on reflected LazySignalsObservable trait object
        world.resource_scope(|world, type_registry: Mut<AppTypeRegistry>| {
            let type_registry = type_registry.read();

            for (entity, component_id) in component_id_set.iter() {
                let entity = *entity;

                // here we need to access the Signal as an LazySignalsObservable
                let component_id = *component_id;
                let mut signal_to_send = world.entity_mut(entity);

                // use the type_id from the component info
                let info = component_info_set.get(component_id).unwrap();
                let type_id = info.type_id().unwrap();
                // the type_id matches the concrete type of the Signal's generic LazySignalsState

                // it comes from ComponentInfo which is retrieved from the ECS world

                // the component_id is saved when the command to make the concrete Signal runs

                // merge the next data value and return a list of subscribers to the change
                // and whether these subscribers should be triggered too
                let result = run_as_observable(
                    &mut signal_to_send,
                    None,
                    None,
                    &component_id,
                    &type_id,
                    &type_registry,
                    Box::new(|observable, _args, _target| { observable.merge() })
                ).unwrap();

                let subs = result.0;
                let changed = result.1;
                let triggered = result.2;

                if changed {
                    signals.changed.insert(entity, ());
                }

                // add subscribers to the running set and mark if triggered
                //info!("SUBS for {:#?} are: {:#?}", entity, subs);
                add_subs_to_running(&subs, triggered, &mut next_running, &mut signals);

                // mark as processed
                signal_to_send.remove::<SendSignal>();
            }

            // Phase Two: fire notifications up the subscriber tree
            let mut count = 0;

            // as long as there is a next_running set, move next_running set into the current one
            while merge_running(&mut running, &mut next_running) {
                count += 1;
                trace!("Sending signals iteration {}", count);

                // get an item from the running set
                for runner in running.indices() {
                    // add the item to the processed set
                    processed.insert(runner, ());

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
                                .unwrap().result_type;
                            trace!(
                                "--got component_id {:?} and type_id {:?}",
                                component_id,
                                type_id
                            );

                            // get a list of subscribers
                            let subs = run_as_observable(
                                &mut subscriber,
                                None,
                                None,
                                &component_id,
                                &type_id,
                                &type_registry,
                                Box::new(|observable, _args, _target| {
                                    Some((observable.get_subscribers(), false, false))
                                })
                            );

                            // computed has its own subscribers, so add those to the next_running set
                            // and mark triggered if appropriate
                            add_subs_to_running(
                                &subs.unwrap().0,
                                signals.triggered.contains(runner),
                                &mut next_running,
                                &mut signals
                            );
                        }
                    }
                }

                // clear the running set at the end of each iteration
                running.clear();
            }
        });
    });
}
