use bevy::prelude::*;

use crate::{ commands::*, signals::* };

/// ## Main Signal primitive factory.
/// Convenience functions for Signal creation and manipulation inspired by the TC39 proposal.
pub struct Signal;
impl Signal {
    pub fn computed<P: SignalsParams, R: SignalsData>(
        &self,
        propagator: Box<dyn PropagatorFn>,
        sources: Vec<Entity>,
        commands: &mut Commands
    ) -> Entity {
        let entity = commands.spawn_empty().id();
        commands.create_computed::<P, R>(entity, propagator, sources);
        entity
    }

    pub fn effect<P: SignalsParams>(
        &self,
        effect: Box<dyn EffectFn>,
        triggers: Vec<Entity>,
        commands: &mut Commands
    ) -> Entity {
        let entity = commands.spawn_empty().id();
        commands.create_effect::<P>(entity, effect, triggers);
        entity
    }

    pub fn read<R: SignalsData>(
        &self,
        immutable: Option<Entity>,
        world: &World
    ) -> SignalsResult<R> {
        match immutable {
            Some(immutable) => {
                let entity = world.entity(immutable);
                match entity.get::<LazyImmutable<R>>() {
                    Some(observable) => observable.read(),

                    // TODO maybe add some kind of config option to ignore errors and return a default
                    None => Err(SignalsError::ReadError(immutable)),
                }
            }
            None => Err(SignalsError::NoSignalError),
        }
    }

    pub fn send<T: SignalsData>(&self, signal: Option<Entity>, data: T, commands: &mut Commands) {
        if let Some(signal) = signal {
            commands.send_signal::<T>(signal, data);
        }
    }

    pub fn state<T: SignalsData>(&self, data: T, commands: &mut Commands) -> Entity {
        let state = commands.spawn_empty().id();
        commands.create_state::<T>(state, data);
        state
    }

    pub fn value<R: SignalsData>(
        &self,
        immutable: Option<Entity>,
        caller: Entity,
        world: &mut World
    ) -> SignalsResult<R> {
        match immutable {
            Some(immutable) => {
                let mut entity = world.entity_mut(immutable);
                match entity.get_mut::<LazyImmutable<R>>() {
                    Some(mut observable) => { observable.value(caller) }

                    // TODO maybe add some kind of config option to ignore errors and return default
                    None => Err(SignalsError::ReadError(immutable)),
                }
            }
            None => Err(SignalsError::NoSignalError),
        }
    }
}
