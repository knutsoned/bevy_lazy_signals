use bevy::{ ecs::world::Command, prelude::* };

use crate::signals::*;

/// Convenience extension to use each Command directly from Commands instance.
pub trait SignalsCommandsExt {
    /// Command to create a computed memo (Immutable plus Propagator) from the given entity.
    fn create_computed<T: Copy + PartialEq + Send + Sync + 'static>(
        &mut self,
        computed: Entity,
        propagator: Box<PropagatorFn>,
        sources: Vec<Entity>,
        init_value: T
    );

    /// Command to create an effect (Propagator with no Immutable) from the given entity.
    fn create_effect(
        &mut self,
        effect: Entity,
        propagator: Box<PropagatorFn>,
        triggers: Vec<Entity>
    );

    /// Command to create a state (Immutable with no Propagator) from the given entity.
    fn create_state<T: Copy + PartialEq + Send + Sync + 'static>(&mut self, state: Entity, data: T);

    fn send_signal<T: Copy + PartialEq + Send + Sync + 'static>(&mut self, signal: Entity, data: T);
}

impl<'w, 's> SignalsCommandsExt for Commands<'w, 's> {
    fn create_computed<T: Copy + PartialEq + Send + Sync + 'static>(
        &mut self,
        computed: Entity,
        propagator: Box<PropagatorFn>,
        sources: Vec<Entity>,
        init_value: T
    ) {
        self.add(CreateComputedCommand::<T> {
            computed,
            propagator,
            sources,
            init_value,
        });
    }

    fn create_effect(
        &mut self,
        effect: Entity,
        propagator: Box<PropagatorFn>,
        triggers: Vec<Entity>
    ) {
        self.add(CreateEffectCommand {
            effect,
            propagator,
            triggers,
        });
    }

    fn create_state<T: Copy + PartialEq + Send + Sync + 'static>(
        &mut self,
        state: Entity,
        data: T
    ) {
        self.add(CreateStateCommand::<T> {
            state,
            data,
        });
    }

    fn send_signal<T: Copy + PartialEq + Send + Sync + 'static>(
        &mut self,
        signal: Entity,
        data: T
    ) {
        self.add(SendSignalCommand::<T> {
            signal,
            data,
        });
    }
}

/// Command to create a computed memo (Immutable plus Propagator) from the given entity.
pub struct CreateComputedCommand<T: Copy + PartialEq + Send + Sync + 'static> {
    computed: Entity,
    propagator: Box<PropagatorFn>,
    sources: Vec<Entity>,
    init_value: T,
}

impl<T: Copy + PartialEq + Send + Sync + 'static> Command for CreateComputedCommand<T> {
    fn apply(self, world: &mut World) {
        let component_id = world.init_component::<LazyImmutable<T>>();
        world
            .get_entity_mut(self.computed)
            .unwrap()
            .insert((
                LazyImmutable::<T>::new(self.init_value),
                ImmutableComponentId { component_id },
                Propagator {
                    propagator: self.propagator,
                    sources: self.sources,
                },
                RebuildSubscribers,
            ));
    }
}

/// Command to create an effect (Propagator with no memo) from the given entity.
pub struct CreateEffectCommand {
    effect: Entity,
    propagator: Box<PropagatorFn>,
    triggers: Vec<Entity>,
}

impl Command for CreateEffectCommand {
    fn apply(self, world: &mut World) {
        world
            .get_entity_mut(self.effect)
            .unwrap()
            .insert((
                Propagator {
                    propagator: self.propagator,
                    sources: self.triggers,
                },
                RebuildSubscribers,
            ));
    }
}

/// Command to create a state (Immutable) from the given entity.
pub struct CreateStateCommand<T: Copy + PartialEq + Send + Sync + 'static> {
    state: Entity,
    data: T,
}

impl<T: Copy + PartialEq + Send + Sync + 'static> Command for CreateStateCommand<T> {
    fn apply(self, world: &mut World) {
        // store the ComponentId so we can reflect the LazyImmutable
        let component_id = world.init_component::<LazyImmutable<T>>();
        world
            .get_entity_mut(self.state)
            .unwrap()
            .insert((LazyImmutable::<T>::new(self.data), ImmutableComponentId { component_id }));
    }
}

/// Command to send a Signal (i.e. update an Immutable during the next tick) to the given entity.
pub struct SendSignalCommand<T: Copy + PartialEq + Send + Sync + 'static> {
    signal: Entity,
    data: T,
}

impl<T: Copy + PartialEq + Send + Sync + 'static> Command for SendSignalCommand<T> {
    fn apply(self, world: &mut World) {
        trace!("SendSignalCommand {:?}", self.signal);
        // we're less sure the signal actually exists, but don't panic if not
        // (assume the caller removed it and we don't care about it anymore)
        if let Some(mut entity) = world.get_entity_mut(self.signal) {
            if let Some(mut immutable) = entity.get_mut::<LazyImmutable<T>>() {
                immutable.merge_next(self.data);
                entity.insert(SendSignal);
                trace!("merged next and inserted SendSignal");
            } else {
                error!("could not get Immutable");
            }
        } else {
            error!("could not get Signal");
        }
    }
}
