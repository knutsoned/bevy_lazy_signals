use bevy::prelude::*;

use crate::{ commands::*, signals::* };

/// ## Main Signal primitive factory.
/// Convenience functions for Signal creation and manipulation inspired by the TC39 proposal.
pub struct Signal;
impl Signal {
    pub fn computed<T: Copy + PartialEq + Send + Sync + 'static>(
        &self,
        propagator: Box<dyn PropagatorFn>,
        sources: Vec<Entity>,
        init_value: T,
        commands: &mut Commands
    ) -> Entity {
        let computed = commands.spawn_empty().id();
        commands.create_computed::<T>(computed, propagator, sources, init_value);
        computed
    }

    pub fn effect(
        &self,
        propagator: Box<dyn EffectFn>,
        triggers: Vec<Entity>,
        commands: &mut Commands
    ) -> Entity {
        let effect = commands.spawn_empty().id();
        commands.create_effect(effect, propagator, triggers);
        effect
    }

    pub fn read<T: Copy + PartialEq + Send + Sync + 'static>(
        &self,
        immutable: Option<Entity>,
        world: &World
    ) -> SignalsResult<T> {
        match immutable {
            Some(immutable) => {
                let entity = world.entity(immutable);
                match entity.get::<LazyImmutable<T>>() {
                    Some(observable) => Ok(observable.read()),

                    // TODO maybe add some kind of config option to ignore errors and return default
                    None => Err(SignalsError::ReadError(immutable)),
                }
            }
            None => Err(SignalsError::NoSignalError),
        }
    }

    pub fn send<T: Copy + PartialEq + Send + Sync + 'static>(
        &self,
        signal: Option<Entity>,
        data: T,
        commands: &mut Commands
    ) {
        if let Some(signal) = signal {
            commands.send_signal::<T>(signal, data);
        }
    }

    pub fn state<T: Copy + PartialEq + Send + Sync + 'static>(
        &self,
        data: T,
        commands: &mut Commands
    ) -> Entity {
        let state = commands.spawn_empty().id();
        commands.create_state::<T>(state, data);
        state
    }

    pub fn value<T: Copy + PartialEq + Send + Sync + 'static>(
        &self,
        immutable: Option<Entity>,
        caller: Entity,
        world: &mut World
    ) -> SignalsResult<T> {
        match immutable {
            Some(immutable) => {
                let mut entity = world.entity_mut(immutable);
                match entity.get_mut::<LazyImmutable<T>>() {
                    Some(mut observable) => { Ok(observable.value(caller)) }

                    // TODO maybe add some kind of config option to ignore errors and return default
                    None => Err(SignalsError::ReadError(immutable)),
                }
            }
            None => Err(SignalsError::NoSignalError),
        }
    }
}
