use std::sync::Mutex;

use bevy::{ prelude::*, reflect::{ DynamicTuple, GetTupleField } };

use crate::{
    arcane_wizardry::make_tuple,
    commands::LazySignalsCommandsExt,
    framework::*,
    lazy_immutable::{ LazySignalsImmutable, LazySignalsState },
};

/// This is the reference user API, patterned after the TC39 proposal.
///
/// Convenience function to get a field directly from a DynamicTuple.
pub fn get_field<T: LazySignalsData>(tuple: &DynamicTuple, index: usize) -> Option<&T> {
    tuple.get_field::<T>(index) // returns None if type doesn't match
}

pub fn make_effect_with<P: LazySignalsArgs>(
    mut closure: impl Effect<P>
) -> Mutex<Box<dyn EffectWrapper>> {
    Mutex::new(
        Box::new(move |tuple, world| {
            trace!("-running effect context with args {:?}", tuple);
            closure(make_tuple::<P>(tuple), world);
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

pub fn make_task_with<P: LazySignalsArgs>(
    closure: impl AsyncTask<P>
) -> Mutex<Box<dyn TaskWrapper>> {
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
    //info!("Storing result {:?} in {:#?}", data, world.inspect_entity(*entity));
    let mut entity = world.entity_mut(*entity);
    let mut component = entity.get_mut::<LazySignalsState<T>>().unwrap();
    component.update(data)
}

/// ## Main Signal primitive factory.
/// Convenience functions for Signal creation and manipulation inspired by the TC39 proposal.
pub struct LazySignals;
impl LazySignals {
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

    pub fn error<T: LazySignalsData>(error: LazySignalsError) -> LazySignalsResult<T> {
        LazySignalsResult { data: None, error: Some(error) }
    }

    // alias for value
    pub fn get<R: LazySignalsData>(&self, immutable: Entity, world: &World) -> Option<R> {
        self.value(immutable, world)
    }

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

    pub fn option<T: LazySignalsData>(data: Option<T>) -> LazySignalsResult<T> {
        LazySignalsResult { data, error: None }
    }

    // alias for value
    pub fn read<R: LazySignalsData>(&self, immutable: Entity, world: &World) -> Option<R> {
        self.value(immutable, world)
    }

    pub fn result<T: LazySignalsData>(data: T) -> LazySignalsResult<T> {
        LazySignalsResult { data: Some(data), error: None }
    }

    pub fn send<T: LazySignalsData>(&self, signal: Entity, data: T, commands: &mut Commands) {
        commands.send_signal::<T>(signal, data);
    }

    pub fn send_and_trigger<T: LazySignalsData>(
        &self,
        signal: Entity,
        data: T,
        commands: &mut Commands
    ) {
        commands.trigger_signal::<T>(signal, data);
    }

    pub fn state<T: LazySignalsData>(&self, data: T, commands: &mut Commands) -> Entity {
        let state = commands.spawn_empty().id();
        commands.create_state::<T>(state, data);
        state
    }

    pub fn task<P: LazySignalsArgs>(
        &self,
        task_closure: impl AsyncTask<P>,
        sources: Vec<Entity>,
        triggers: Vec<Entity>,
        commands: &mut Commands
    ) -> Entity {
        let entity = commands.spawn_empty().id();
        commands.create_task::<P>(entity, make_task_with(task_closure), sources, triggers);
        entity
    }

    pub fn trigger(&self, signal: Entity, commands: &mut Commands) {
        commands.trigger_signal::<()>(signal, ());
    }

    pub fn value<R: LazySignalsData>(&self, immutable: Entity, world: &World) -> Option<R> {
        let entity = world.entity(immutable);
        match entity.get::<LazySignalsState<R>>() {
            Some(observable) => observable.get(),
            None => None,
        }
    }
}
