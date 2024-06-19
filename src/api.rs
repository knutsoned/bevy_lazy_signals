use std::sync::Mutex;

use bevy::{ ecs::system::BoxedSystem, prelude::* };

use crate::{
    arcane_wizardry::make_tuple,
    commands::LazySignalsCommandsExt,
    framework::*,
    lazy_immutable::{ LazySignalsImmutable, LazySignalsState },
};

/// This is the reference user API, patterned after the TC39 proposal.
pub fn make_effect_with<P: LazySignalsArgs>(
    mut closure: impl Effect<P>
) -> Mutex<Box<dyn EffectWrapper>> {
    Mutex::new(
        Box::new(move |tuple, world| {
            trace!("-running effect context with args {:?}", tuple);
            closure(make_tuple::<P>(tuple), world)
        })
    )
}

pub fn make_computed_with<P: LazySignalsArgs, R: LazySignalsData>(
    closure: impl Computed<P, R>
) -> Mutex<Box<dyn ComputedContext>> {
    Mutex::new(
        Box::new(move |tuple, entity, world| {
            trace!("-running computed context with args {:?}", tuple);
            let result = closure(make_tuple::<P>(tuple));
            if let Some(error) = result.error {
                // TODO process errors
                error!("ERROR running computed: {}", error.to_string());
            }
            store_result::<R>(result, entity, world)
        })
    )
}

pub fn make_action_with<P: LazySignalsArgs>(
    closure: impl Action<P>
) -> Mutex<Box<dyn ActionWrapper>> {
    Mutex::new(
        Box::new(move |tuple| {
            trace!("-running task context with args {:?}", tuple);
            closure(make_tuple::<P>(tuple))
        })
    )
}

/// Convenience function to store a result in an entity.
pub fn store_result<T: LazySignalsData>(
    data: LazySignalsResult<T>,
    entity: &Entity,
    world: &mut World
) -> bool {
    let mut entity = world.entity_mut(*entity);
    let mut component = entity.get_mut::<LazySignalsState<T>>().unwrap();
    component.update(data)
}

/// ## Main Signal primitive factory.
/// Convenience functions for Signal creation and manipulation inspired by the TC39 proposal.
pub struct LazySignals;
impl LazySignals {
    /// Create an Action that will run as an AsyncTask.
    pub fn action<P: LazySignalsArgs>(
        &self,
        task_closure: impl Action<P>,
        sources: Vec<Entity>,
        triggers: Vec<Entity>,
        commands: &mut Commands
    ) -> Entity {
        let entity = commands.spawn_empty().id();
        commands.create_action::<P>(entity, make_action_with(task_closure), sources, triggers);
        entity
    }

    /// Create a BoxedSystem to be chained after the Effect that returns it.
    pub fn box_system<M>(&self, effect_system: impl IntoSystem<(), (), M>) -> Option<BoxedSystem> {
        Some(Box::new(IntoSystem::into_system(effect_system)))
    }

    /// Create a Computed that passes its sources to and evaluate a closure, memoizing the result.
    pub fn computed<P: LazySignalsArgs, R: LazySignalsData>(
        &self,
        propagator_closure: impl Computed<P, R>,
        sources: Vec<Entity>,
        commands: &mut Commands
    ) -> Entity {
        let entity = commands.spawn_empty().id();
        commands.create_computed::<P, R>(entity, make_computed_with(propagator_closure), sources);
        entity
    }

    /// TODO have this return a tuple of getter fn and Src object.
    pub fn computed_tuple<P: LazySignalsArgs, R: LazySignalsData>(
        &self,
        propagator_closure: impl Computed<P, R>,
        sources: Box<impl LazySignalsSources<P>>,
        commands: &mut Commands
    ) -> Entity {
        let entity = commands.spawn_empty().id();
        // FIXME I think this requires a macro
        // but how do we pass in a tuple type and convert that to tuple(Option<EachType>, ...) elsewhere then???
        commands.create_computed::<P, R>(entity, make_computed_with(propagator_closure), sources);
        entity
    }

    /// Create an Effect that passes its sources to and evaluate a closure that runs side-effects.
    pub fn effect<P: LazySignalsArgs>(
        &self,
        effect_closure: impl Effect<P>,
        sources: Vec<Entity>,
        triggers: Vec<Entity>,
        commands: &mut Commands
    ) -> Entity {
        let entity = commands.spawn_empty().id();
        commands.create_effect::<P>(entity, make_effect_with(effect_closure), sources, triggers);
        entity
    }

    /// Return an error from a computed closure.
    pub fn error<T: LazySignalsData>(error: LazySignalsError) -> LazySignalsResult<T> {
        LazySignalsResult { data: None, error: Some(error) }
    }

    /// Alias for value.
    pub fn get<R: LazySignalsData>(&self, immutable: Entity, world: &World) -> Option<R> {
        self.value(immutable, world)
    }

    /// Check the given entity for an error.
    pub fn get_error<R: LazySignalsData>(
        &self,
        immutable: Entity,
        world: &World
    ) -> Option<LazySignalsError> {
        let entity = world.entity(immutable);
        match entity.get::<LazySignalsState<R>>() {
            Some(observable) => observable.error(),
            None => None,
        }
    }

    /// Return an optional value from a computed closure.
    pub fn option<T: LazySignalsData>(data: Option<T>) -> LazySignalsResult<T> {
        LazySignalsResult { data, error: None }
    }

    /// Alias for value.
    pub fn read<R: LazySignalsData>(&self, immutable: Entity, world: &World) -> Option<R> {
        self.value(immutable, world)
    }

    /// Return a value from a computed closure.
    pub fn result<T: LazySignalsData>(data: T) -> LazySignalsResult<T> {
        LazySignalsResult { data: Some(data), error: None }
    }

    /// Send a signal to be applied during the next batch.
    pub fn send<T: LazySignalsData>(&self, signal: Entity, data: T, commands: &mut Commands) {
        commands.send_signal::<T>(signal, data);
    }

    /// Send a signal to be applied during the next batch regardless of whether the data changed.
    pub fn send_and_trigger<T: LazySignalsData>(
        &self,
        signal: Entity,
        data: T,
        commands: &mut Commands
    ) {
        commands.trigger_signal::<T>(signal, data);
    }

    /// Create a Signal state that is the entrypoint for data into the structure.
    pub fn state<T: LazySignalsData>(&self, data: T, commands: &mut Commands) -> Entity {
        let state = commands.spawn_empty().id();
        commands.create_state::<T>(state, data);
        state
    }

    /// TODO have this return a tuple of getter/setter fns and a Src object.
    pub fn state_tuple<T: LazySignalsData>(&self, data: T, commands: &mut Commands) -> Entity {
        let state = commands.spawn_empty().id();
        commands.create_state::<T>(state, data);
        state
    }

    /// Trigger a Signal that takes the unit type as its generic param..
    pub fn trigger(&self, signal: Entity, commands: &mut Commands) {
        commands.trigger_signal::<()>(signal, ());
    }

    /// Get the value from the given World.
    pub fn value<R: LazySignalsData>(&self, immutable: Entity, world: &World) -> Option<R> {
        let entity = world.entity(immutable);
        match entity.get::<LazySignalsState<R>>() {
            Some(observable) => observable.get(),
            None => None,
        }
    }
}
