use bevy::{ prelude::*, reflect::{ DynamicTuple, GetTupleField } };

use crate::{
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

pub fn make_effect_with<P: LazySignalsParams>(
    mut closure: Box<dyn Effect<P>>
) -> Box<dyn EffectContext> {
    Box::new(move |tuple, world| {
        trace!("-running effect context with params {:?}", tuple);
        closure(make_tuple::<P>(tuple), world);
    })
}

pub fn make_propagator_with<P: LazySignalsParams, R: LazySignalsData>(
    closure: Box<dyn Propagator<P, R>>
) -> Box<dyn PropagatorContext> {
    Box::new(move |tuple, entity, world| {
        trace!("-running propagator context with params {:?}", tuple);
        let result = closure(make_tuple::<P>(tuple));
        if let Some(Err(error)) = result {
            // TODO process errors
            error!("ERROR running propagator: {}", error.to_string());
        }
        store_result::<R>(result, entity, world)
    })
}

/// Convenience function to convert DynamicTuples into a concrete type.
pub fn make_tuple<T: LazySignalsParams>(tuple: &DynamicTuple) -> T {
    <T as FromReflect>::from_reflect(tuple).unwrap()
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
    pub fn computed<P: LazySignalsParams, R: LazySignalsData>(
        &self,
        propagator_closure: Box<dyn Propagator<P, R>>,
        sources: Vec<Entity>,
        commands: &mut Commands
    ) -> Entity {
        let entity = commands.spawn_empty().id();
        commands.create_computed::<P, R>(entity, make_propagator_with(propagator_closure), sources);
        entity
    }

    pub fn effect<P: LazySignalsParams>(
        &self,
        effect_closure: Box<dyn Effect<P>>,
        sources: Vec<Entity>,
        triggers: Vec<Entity>,
        commands: &mut Commands
    ) -> Entity {
        let entity = commands.spawn_empty().id();
        commands.create_effect::<P>(entity, make_effect_with(effect_closure), sources, triggers);
        entity
    }

    pub fn read<R: LazySignalsData>(
        &self,
        immutable: Option<Entity>,
        world: &World
    ) -> LazySignalsResult<R> {
        self.value(immutable, world)
    }

    pub fn send<T: LazySignalsData>(
        &self,
        signal: Option<Entity>,
        data: T,
        commands: &mut Commands
    ) {
        if let Some(signal) = signal {
            commands.send_signal::<T>(signal, data);
        }
    }

    pub fn send_and_trigger<T: LazySignalsData>(
        &self,
        signal: Option<Entity>,
        data: T,
        commands: &mut Commands
    ) {
        if let Some(signal) = signal {
            commands.trigger_signal::<T>(signal, data);
        }
    }

    pub fn state<T: LazySignalsData>(&self, data: T, commands: &mut Commands) -> Entity {
        let state = commands.spawn_empty().id();
        commands.create_state::<T>(state, data);
        state
    }

    pub fn trigger(&self, signal: Option<Entity>, commands: &mut Commands) {
        if let Some(signal) = signal {
            commands.trigger_signal::<()>(signal, ());
        }
    }

    pub fn value<R: LazySignalsData>(
        &self,
        immutable: Option<Entity>,
        world: &World
    ) -> LazySignalsResult<R> {
        match immutable {
            Some(immutable) => {
                let entity = world.entity(immutable);
                match entity.get::<LazySignalsState<R>>() {
                    Some(observable) => observable.value(),

                    // TODO maybe add some kind of config option to ignore errors and return a default
                    None => Some(Err(LazySignalsError::ReadError(immutable))),
                }
            }
            None => Some(Err(LazySignalsError::NoSignalError)),
        }
    }
}
