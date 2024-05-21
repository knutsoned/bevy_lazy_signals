use std::marker::PhantomData;

use bevy::{ ecs::world::Command, prelude::* };

use crate::signals::*;

/// Convenience extension to use each Command directly from Commands instance.
pub trait SignalsCommandsExt {
    /// Command to create a computed memo (Immutable plus Propagator) from the given entity.
    fn create_computed<P: SignalsParams, R: SignalsData>(
        &mut self,
        computed: Entity,
        function: Box<dyn PropagatorFn>,
        sources: Vec<Entity>
    );

    /// Command to create an effect (Propagator with no Immutable) from the given entity.
    fn create_effect<P: SignalsParams>(
        &mut self,
        effect: Entity,
        function: Box<dyn EffectFn>,
        triggers: Vec<Entity>
    );

    /// Command to create a state (Immutable with no Propagator) from the given entity.
    fn create_state<T: SignalsData>(&mut self, state: Entity, data: T);

    fn send_signal<T: SignalsData>(&mut self, signal: Entity, data: T);

    fn trigger_signal<T: SignalsData>(&mut self, signal: Entity, data: T);
}

impl<'w, 's> SignalsCommandsExt for Commands<'w, 's> {
    fn create_computed<P: SignalsParams, R: SignalsData>(
        &mut self,
        computed: Entity,
        function: Box<dyn PropagatorFn>,
        sources: Vec<Entity>
    ) {
        self.add(CreateComputedCommand::<P, R> {
            computed,
            function,
            sources,
            param_tuple: PhantomData,
            result_type: PhantomData,
        });
    }

    fn create_effect<P: SignalsParams>(
        &mut self,
        effect: Entity,
        function: Box<dyn EffectFn>,
        triggers: Vec<Entity>
    ) {
        self.add(CreateEffectCommand::<P> {
            effect,
            function,
            triggers,
            param_tuple: PhantomData,
        });
    }

    fn create_state<T: SignalsData>(&mut self, state: Entity, data: T) {
        self.add(CreateStateCommand {
            state,
            data,
        });
    }

    fn send_signal<T: SignalsData>(&mut self, signal: Entity, data: T) {
        self.add(SendSignalCommand {
            signal,
            data,
        });
    }

    fn trigger_signal<T: SignalsData>(&mut self, signal: Entity, data: T) {
        self.add(TriggerSignalCommand {
            signal,
            data,
        });
    }
}

/// Command to create a computed memo (Immutable plus Propagator) from the given entity.
pub struct CreateComputedCommand<P: SignalsParams, R: SignalsData> {
    computed: Entity,
    function: Box<dyn PropagatorFn>,
    sources: Vec<Entity>,
    param_tuple: PhantomData<P>,
    result_type: PhantomData<R>,
}

impl<P: SignalsParams, R: SignalsData> Command for CreateComputedCommand<P, R> {
    fn apply(self, world: &mut World) {
        let component_id = world.init_component::<LazyImmutable<R>>();
        world
            .get_entity_mut(self.computed)
            .unwrap()
            .insert((
                LazyImmutable::<R>::new(Err(SignalsError::NoValue)),
                ImmutableComponentId { component_id },
                Propagator {
                    function: self.function,
                    sources: self.sources,
                },
                RebuildSubscribers,
            ));
    }
}

/// Command to create an effect (Propagator with no memo) from the given entity.
pub struct CreateEffectCommand<P: SignalsParams> {
    effect: Entity,
    function: Box<dyn EffectFn>,
    triggers: Vec<Entity>,
    param_tuple: PhantomData<P>,
}

impl<P: SignalsParams> Command for CreateEffectCommand<P> {
    fn apply(self, world: &mut World) {
        world
            .get_entity_mut(self.effect)
            .unwrap()
            .insert((
                Effect {
                    function: self.function,
                    triggers: self.triggers,
                },
                RebuildSubscribers,
            ));
    }
}

/// Command to create a state (Immutable) from the given entity.
pub struct CreateStateCommand<T: SignalsData> {
    state: Entity,
    data: T,
}

impl<T: SignalsData> Command for CreateStateCommand<T> {
    fn apply(self, world: &mut World) {
        // store the ComponentId so we can reflect the LazyImmutable
        let component_id = world.init_component::<LazyImmutable<T>>();
        world
            .get_entity_mut(self.state)
            .unwrap()
            .insert((
                LazyImmutable::<T>::new(Ok(self.data)),
                ImmutableComponentId { component_id },
            ));
    }
}

/// Command to send a Signal (i.e. update an Immutable during the next tick) to the given entity.
pub struct SendSignalCommand<T: SignalsData> {
    signal: Entity,
    data: T,
}

impl<T: SignalsData> Command for SendSignalCommand<T> {
    fn apply(self, world: &mut World) {
        trace!("SendSignalCommand {:?}", self.signal);
        // we're less sure the signal actually exists, but don't panic if not
        // (assume the caller removed it and we don't care about it anymore)
        if let Some(mut entity) = world.get_entity_mut(self.signal) {
            if let Some(mut immutable) = entity.get_mut::<LazyImmutable<T>>() {
                immutable.merge_next(Ok(self.data), false);
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

/// Command to trigger a Signal (i.e. send signal even if value unchanged) to the given entity.
pub struct TriggerSignalCommand<T: SignalsData> {
    signal: Entity,
    data: T,
}

impl<T: SignalsData> Command for TriggerSignalCommand<T> {
    fn apply(self, world: &mut World) {
        trace!("TriggerSignalCommand {:?}", self.signal);
        // we're less sure the signal actually exists, but don't panic if not
        // (assume the caller removed it and we don't care about it anymore)
        if let Some(mut entity) = world.get_entity_mut(self.signal) {
            if let Some(mut immutable) = entity.get_mut::<LazyImmutable<T>>() {
                immutable.merge_next(Ok(self.data), true);
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
