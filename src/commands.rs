use std::{ marker::PhantomData, sync::Mutex };

use bevy::{ ecs::world::Command, prelude::* };

use crate::{ bundles::*, framework::*, lazy_immutable::{ LazySignalsState, LazySignalsImmutable } };

/// Convenience extension to use each Command directly from Commands instance.
pub trait LazySignalsCommandsExt {
    /// Command to create a computed memo from the given entity.
    fn create_computed<P: LazySignalsArgs, R: LazySignalsData>(
        &mut self,
        computed: Entity,
        function: Mutex<Box<dyn ComputedContext>>,
        sources: Vec<Entity>
    );

    /// Command to create a short-lived effect from the given entity.
    fn create_effect<P: LazySignalsArgs>(
        &mut self,
        effect: Entity,
        function: Mutex<Box<dyn EffectWrapper>>,
        sources: Vec<Entity>,
        triggers: Vec<Entity>
    );

    /// Command to create a state (LazyImmutable with no Effect or Propagator) from the given entity.
    fn create_state<T: LazySignalsData>(&mut self, state: Entity, data: T);

    /// Command to create an effect from the given entity as an async task.
    fn create_task<P: LazySignalsArgs>(
        &mut self,
        effect: Entity,
        function: Mutex<Box<dyn TaskWrapper>>,
        sources: Vec<Entity>,
        triggers: Vec<Entity>
    );

    // Command to send a signal if the data value is different from the current value.
    fn send_signal<T: LazySignalsData>(&mut self, signal: Entity, data: T);

    // Command to send a signal even if the data value is unchanged.
    fn trigger_signal<T: LazySignalsData>(&mut self, signal: Entity, data: T);
}

impl<'w, 's> LazySignalsCommandsExt for Commands<'w, 's> {
    fn create_computed<P: LazySignalsArgs, R: LazySignalsData>(
        &mut self,
        computed: Entity,
        function: Mutex<Box<dyn ComputedContext>>,
        sources: Vec<Entity>
    ) {
        self.add(CreateComputedCommand::<P, R> {
            computed,
            function,
            sources,
            args_type: PhantomData,
            result_type: PhantomData,
        });
    }

    fn create_effect<P: LazySignalsArgs>(
        &mut self,
        effect: Entity,
        function: Mutex<Box<dyn EffectWrapper>>,
        sources: Vec<Entity>,
        triggers: Vec<Entity>
    ) {
        self.add(CreateEffectCommand::<P> {
            effect,
            function,
            sources,
            triggers,
            args_type: PhantomData,
        });
    }

    fn create_state<T: LazySignalsData>(&mut self, state: Entity, data: T) {
        self.add(CreateStateCommand {
            state,
            data,
        });
    }

    fn create_task<P: LazySignalsArgs>(
        &mut self,
        effect: Entity,
        function: Mutex<Box<dyn TaskWrapper>>,
        sources: Vec<Entity>,
        triggers: Vec<Entity>
    ) {
        self.add(CreateTaskCommand::<P> {
            effect,
            function,
            sources,
            triggers,
            args_type: PhantomData,
        });
    }

    fn send_signal<T: LazySignalsData>(&mut self, signal: Entity, data: T) {
        self.add(SendSignalCommand {
            signal,
            data,
        });
    }

    fn trigger_signal<T: LazySignalsData>(&mut self, signal: Entity, data: T) {
        self.add(TriggerSignalCommand {
            signal,
            data,
        });
    }
}

/// Command to create a computed memo (Immutable plus Propagator) from the given entity.
pub struct CreateComputedCommand<P: LazySignalsArgs, R: LazySignalsData> {
    pub computed: Entity,
    pub function: Mutex<Box<dyn ComputedContext>>,
    pub sources: Vec<Entity>,
    pub args_type: PhantomData<P>,
    pub result_type: PhantomData<R>,
}

impl<P: LazySignalsArgs, R: LazySignalsData> Command for CreateComputedCommand<P, R> {
    fn apply(self, world: &mut World) {
        // once init runs once for a concrete R, it just returns the existing ComponentId next time
        let component_id = world.init_component::<LazySignalsState<R>>();
        world
            .get_entity_mut(self.computed)
            .unwrap()
            .insert(
                ComputedBundle::<R>::from_function::<P>(self.function, self.sources, component_id)
            );
    }
}

/// Command to create an effect (Propagator with no memo) from the given entity.
pub struct CreateEffectCommand<P: LazySignalsArgs> {
    pub effect: Entity,
    pub function: Mutex<Box<dyn EffectWrapper>>,
    pub sources: Vec<Entity>,
    pub triggers: Vec<Entity>,
    pub args_type: PhantomData<P>,
}

impl<P: LazySignalsArgs> Command for CreateEffectCommand<P> {
    fn apply(self, world: &mut World) {
        world
            .get_entity_mut(self.effect)
            .unwrap()
            .insert(
                EffectBundle::from_function::<P>(
                    EffectContext::Short(self.function),
                    self.sources,
                    self.triggers
                )
            );
    }
}

/// Command to create a state (LazyImmutableImmutable) from the given entity.
pub struct CreateStateCommand<T: LazySignalsData> {
    pub state: Entity,
    pub data: T,
}

impl<T: LazySignalsData> Command for CreateStateCommand<T> {
    fn apply(self, world: &mut World) {
        // store the ComponentId so we can reflect the LazyImmutable later
        let component_id = world.init_component::<LazySignalsState<T>>();
        world
            .get_entity_mut(self.state)
            .unwrap()
            .insert(StateBundle::<T>::from_value(self.data, component_id));
    }
}

/// Command to create a task (non-blocking effect) from the given entity.
pub struct CreateTaskCommand<P: LazySignalsArgs> {
    pub effect: Entity,
    pub function: Mutex<Box<dyn TaskWrapper>>,
    pub sources: Vec<Entity>,
    pub triggers: Vec<Entity>,
    pub args_type: PhantomData<P>,
}

impl<P: LazySignalsArgs> Command for CreateTaskCommand<P> {
    fn apply(self, world: &mut World) {
        world
            .get_entity_mut(self.effect)
            .unwrap()
            .insert(
                EffectBundle::from_function::<P>(
                    EffectContext::Long(self.function),
                    self.sources,
                    self.triggers
                )
            );
    }
}

/// Command to send a Signal (i.e. update a LazyImmutable during the next tick) to the given entity.
pub struct SendSignalCommand<T: LazySignalsData> {
    pub signal: Entity,
    pub data: T,
}

impl<T: LazySignalsData> Command for SendSignalCommand<T> {
    fn apply(self, world: &mut World) {
        trace!("SendSignalCommand {:?}", self.signal);
        // we're less sure the signal actually exists, but don't panic if not
        // (assume the caller removed it and we don't care about it anymore)
        if let Some(mut entity) = world.get_entity_mut(self.signal) {
            if let Some(mut immutable) = entity.get_mut::<LazySignalsState<T>>() {
                immutable.merge_next(
                    LazySignalsResult { data: Some(self.data), error: None },
                    false
                );
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
pub struct TriggerSignalCommand<T: LazySignalsData> {
    pub signal: Entity,
    pub data: T,
}

impl<T: LazySignalsData> Command for TriggerSignalCommand<T> {
    fn apply(self, world: &mut World) {
        trace!("TriggerSignalCommand {:?}", self.signal);
        // we're less sure the signal actually exists, but don't panic if not
        // (assume the caller removed it and we don't care about it anymore)
        if let Some(mut entity) = world.get_entity_mut(self.signal) {
            if let Some(mut immutable) = entity.get_mut::<LazySignalsState<T>>() {
                immutable.merge_next(
                    LazySignalsResult { data: Some(self.data), error: None },
                    true
                );
                entity.insert(SendSignal);
                trace!("merged next and inserted SendSignal");
            } else {
                error!("could not get State");
            }
        } else {
            error!("could not get Signal");
        }
    }
}
