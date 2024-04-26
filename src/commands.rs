use std::marker::PhantomData;

use bevy_ecs::{ prelude::*, world::Command };

use crate::signals::*;

/// Convenience extension to use each Command directly from Commands instance.
pub trait SignalsCommandsExt {
    /// Command to create a computed memo (Immutable plus Propagator) from the given entity.
    fn create_computed<T: Copy + Default + PartialEq + Send + Sync + 'static>(
        &mut self,
        computed: Entity,
        propagator: Box<dyn PropagatorFn>,
        sources: Vec<Entity>
    );

    /// Command to create an effect (Propagator with no Immutable) from the given entity.
    fn create_effect(
        &mut self,
        effect: Entity,
        propagator: Box<dyn PropagatorFn>,
        triggers: Vec<Entity>
    );

    /// Command to create a state (Immutable with no Propagator) from the given entity.
    fn create_state<T: Copy + Default + PartialEq + Send + Sync + 'static>(
        &mut self,
        state: Entity,
        data: T
    );

    fn send_signal<T: Copy + Default + PartialEq + Send + Sync + 'static>(
        &mut self,
        signal: Entity,
        data: T
    );
}

impl<'w, 's> SignalsCommandsExt for Commands<'w, 's> {
    fn create_computed<T: Copy + Default + PartialEq + Send + Sync + 'static>(
        &mut self,
        computed: Entity,
        propagator: Box<dyn PropagatorFn>,
        sources: Vec<Entity>
    ) {
        self.add(CreateComputedCommand::<T> {
            computed,
            propagator,
            sources,
            phantom_zone: Default::default(),
        });
    }

    fn create_effect(
        &mut self,
        effect: Entity,
        propagator: Box<dyn PropagatorFn>,
        triggers: Vec<Entity>
    ) {
        self.add(CreateEffectCommand {
            effect,
            propagator,
            triggers,
        });
    }

    fn create_state<T: Copy + Default + PartialEq + Send + Sync + 'static>(
        &mut self,
        state: Entity,
        data: T
    ) {
        self.add(CreateStateCommand::<T> {
            state,
            data,
        });
    }

    fn send_signal<T: Copy + Default + PartialEq + Send + Sync + 'static>(
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
pub struct CreateComputedCommand<T: Copy + Default + PartialEq + Send + Sync + 'static> {
    computed: Entity,
    propagator: Box<dyn PropagatorFn>,
    sources: Vec<Entity>,
    phantom_zone: PhantomData<T>,
}

impl<T: Copy + Default + PartialEq + Send + Sync + 'static> Command for CreateComputedCommand<T> {
    fn apply(self, world: &mut World) {
        world
            .get_entity_mut(self.computed)
            .unwrap()
            .insert((
                LazyImmutable::<T>::new(T::default()),
                Propagator {
                    propagator: self.propagator,
                    sources: self.sources,
                },
            ));
    }
}

/// Command to create an effect (Propagator with no memo) from the given entity.
pub struct CreateEffectCommand {
    effect: Entity,
    propagator: Box<dyn PropagatorFn>,
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
            ));
    }
}

/// Command to create a state (Immutable) from the given entity.
pub struct CreateStateCommand<T: Copy + Default + PartialEq + Send + Sync + 'static> {
    state: Entity,
    data: T,
}

impl<T: Copy + Default + PartialEq + Send + Sync + 'static> Command for CreateStateCommand<T> {
    fn apply(self, world: &mut World) {
        world
            .get_entity_mut(self.state)
            .unwrap()
            .insert((LazyImmutable::<T>::new(self.data),));
    }
}

/// Command to send a signal (i.e. update an Immutable during the next tick) to the given entity.
pub struct SendSignalCommand<T: Copy + Default + PartialEq + Send + Sync + 'static> {
    signal: Entity,
    data: T,
}

impl<T: Copy + Default + PartialEq + Send + Sync + 'static> Command for SendSignalCommand<T> {
    fn apply(self, world: &mut World) {
        // we're less sure the signal actually exists, but don't panic if not
        // (assume the caller removed it and we don't care about it anymore)
        // TODO should this panic instead?
        if let Some(mut entity) = world.get_entity_mut(self.signal) {
            if let Some(mut immutable) = entity.get_mut::<LazyImmutable<T>>() {
                immutable.merge_next(self.data);
                entity.insert(SendSignal);
            }
        }
    }
}
