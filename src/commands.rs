use bevy::{ ecs::world::Command, prelude::* };

use crate::signals::*;

/// Convenience extension to use each Command directly from Commands instance.
pub trait SignalsCommandsExt {
    /// Command to create a computed memo (Immutable plus Propagator) from the given entity.
    fn create_computed<T: SignalsData>(
        &mut self,
        computed: Entity,
        function: Box<dyn PropagatorFn>,
        sources: Vec<Entity>,
        init_value: T
    );

    /// Command to create an effect (Propagator with no Immutable) from the given entity.
    fn create_effect(&mut self, effect: Entity, function: Box<dyn EffectFn>, triggers: Vec<Entity>);

    /// Command to create a state (Immutable with no Propagator) from the given entity.
    fn create_state<T: SignalsData>(&mut self, state: Entity, data: T);

    fn send_signal<T: SignalsData>(&mut self, signal: Entity, data: T);
}

impl<'w, 's> SignalsCommandsExt for Commands<'w, 's> {
    fn create_computed<T: SignalsData>(
        &mut self,
        computed: Entity,
        function: Box<dyn PropagatorFn>,
        sources: Vec<Entity>,
        init_value: T
    ) {
        self.add(CreateComputedCommand {
            computed,
            function,
            sources,
            init_value,
        });
    }

    fn create_effect(
        &mut self,
        effect: Entity,
        function: Box<dyn EffectFn>,
        triggers: Vec<Entity>
    ) {
        self.add(CreateEffectCommand {
            effect,
            function,
            triggers,
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
}

/// Command to create a computed memo (Immutable plus Propagator) from the given entity.
pub struct CreateComputedCommand<T: SignalsData> {
    computed: Entity,
    function: Box<dyn PropagatorFn>,
    sources: Vec<Entity>,
    init_value: T,
}

impl<T: SignalsData> Command for CreateComputedCommand<T> {
    fn apply(self, world: &mut World) {
        let component_id = world.init_component::<LazyImmutable<T>>();
        world
            .get_entity_mut(self.computed)
            .unwrap()
            .insert((
                LazyImmutable::<T>::new(self.init_value),
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
pub struct CreateEffectCommand {
    effect: Entity,
    function: Box<dyn EffectFn>,
    triggers: Vec<Entity>,
}

impl Command for CreateEffectCommand {
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
            .insert((LazyImmutable::<T>::new(self.data), ImmutableComponentId { component_id }));
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
