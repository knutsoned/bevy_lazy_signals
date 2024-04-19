use std::marker::PhantomData;

use bevy_ecs::{ prelude::*, system::BoxedSystem };

/// From bevy_rx:
/// Contains all reactive state. A bevy world is used because it makes it easy to store statically
/// typed data in a type erased container.
#[derive(Resource)]
pub struct ReactiveContext<S> {
    pub reactive_state: World,
    pub outside_state: PhantomData<S>,
}

/// From bevy_rx RxObservableData:
/// The core reactive primitive that holds data, and a list of subscribers that are invoked when the
/// data changes.
#[derive(Component)]
pub struct Mutable<T> {
    pub data: T,
    pub subscribers: Vec<Entity>,
}

pub trait Observable {
    type DataType;
    fn reactive_entity(&self) -> Entity;
}

/// A reactive component that can updated with new values or read through the [`ReactiveContext`].
#[derive(Debug, Component)]
pub struct Signal<T: Send + Sync + 'static> {
    pub reactive_entity: Entity,
    pub p: PhantomData<T>,
}

/// A reactive calculation that is run on observable data, and memoized (cached).
///
/// This component lives in the reactive world and holds the user calculation function. [`Memo`] is
/// the user-facing counterpart in the main world, which is a lightweight handle to access this
/// mirror component.
///
/// This component is expected to be on an entity with an [`crate::Mutable`] component. The
/// contained function can be called without the caller knowing any type information, and will
/// update the associated [`Mutable`] component.
/// A reactive value that is automatically recalculated and memoized (cached).
///
/// The value can only be read through the [`ReactiveContext`].
#[derive(Debug, Component)]
pub struct Memo<T: Send + Sync + 'static> {
    pub reactive_entity: Entity,
    pub p: PhantomData<T>,
}

/// Implemented on tuples to be used for querying
pub trait MemoQuery<T>: Copy + Send + Sync + 'static {
    type Query<'a>;
    fn read_and_derive(
        world: &mut World,
        reader: Entity,
        derive_fn: impl Fn(Self::Query<'_>) -> T,
        input_deps: Self
    ) -> Option<T>;
}

pub trait DeriveFn: Send + Sync + FnMut(&mut World, &mut Vec<Entity>) {}

#[derive(Component)]
pub struct RxMemo {
    pub function: Box<dyn DeriveFn>,
}

/// A reactive component that makes changes to the bevy [`World`] by running a system only when the
/// values it queries change.
#[derive(Debug, Clone, Copy, Component)]
pub struct Effect {
    pub reactive_entity: Entity,
}

/// A side effect applied to the main world at a deferred sync point, as a reaction to some value
/// changing.
///
/// An effect is an optional component stored alongside observable data
/// ([`crate::RxObservableData`]), and is triggered any time that observable data changes.
#[derive(Debug, Component)]
pub struct DeferredEffect {
    pub system: EffectSystem,
}

/// A stack of side effects that is gathered while systems run and update reactive data. This allows
/// effects to be gathered during normal (non-exclusive) system execution in the user's main world.
/// Once the user wants to execute the side effects, the plugin will need an exclusive system to run
/// the effects in a big batch. This is the "deferred" part of the name.
#[derive(Resource, Default)]
pub struct DeferredEffects {
    pub stack: Vec<Box<EffectFn>>,
}

/// A function used to run effects via dependency injection.
pub type EffectFn = dyn FnOnce(&mut World, &mut World) + Send + Sync;

#[derive(Default, Debug)]
pub enum EffectSystem {
    #[default]
    Empty,
    New(BoxedSystem),
    Initialized(BoxedSystem),
}
