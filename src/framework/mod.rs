use std::{ any::TypeId, fmt::Debug };

use bevy::{
    ecs::component::ComponentId,
    prelude::*,
    reflect::{ DynamicTuple, GetTypeRegistration, Tuple },
};

use thiserror::Error;

use crate::LazySignalsObservable;

pub mod lazy_immutable;

/// # Signals framework
/// ## Types
/// Result type for handling error conditions in consumer code.
pub type LazySignalsResult<R> = Option<Result<R, LazySignalsError>>;

/// Return type for returning an optional list of entities and a flag.
pub type MaybeFlaggedEntities = Option<(Vec<Entity>, bool, bool)>;

/// ## Enums
/// Read error.
#[derive(Error, Clone, Copy, PartialEq, Reflect, Debug)]
pub enum LazySignalsError {
    /// An attempt was made to reference a Signal entity that does not have the right components.
    #[error["Signal does not exist"]]
    NoSignalError,

    /// When there is no next value, and we don't want to clobber the existing one (not really err)
    #[error["No next value"]]
    NoNextValue,

    /// An attempt was made to read a signal and something weird went wrong.
    #[error("Error reading signal {0}")]
    ReadError(Entity),
}

// ## Traits
/// An item of data for use with Immutables.
pub trait LazySignalsData: Copy +
    Debug +
    FromReflect +
    GetTypeRegistration +
    PartialEq +
    Reflect +
    Send +
    Sync +
    TypePath +
    'static {}
impl<T> LazySignalsData
    for T
    where
        T: Copy +
            Debug +
            FromReflect +
            GetTypeRegistration +
            PartialEq +
            Reflect +
            Send +
            Sync +
            TypePath +
            'static {}

/// A tuple containing parameters for a computed memo or effect.
pub trait LazySignalsParams: LazySignalsData + Tuple {}
impl<T> LazySignalsParams for T where T: LazySignalsData + Tuple {}

/// A Propagator function aggregates (merges) data from multiple cells to store in a bound cell.
/// Compared to the MIT model, these Propagators pull data into a cell they are bound to.
/// MIT Propagators are conceptually more independent and closer to a push-based flow.
/// This Propagator merges the values of cells denoted by the entity vector into the target entity.
/// It should call value instead of read to make sure it is re-subscribed to its sources!
/// If the target entity is not supplied, the function is assumed to execute side effects only.
///
/// The DynamicTuple is an argument list whose internal types match the Option<T> of each source.
/// The entity is where the result will be stored, where this instance of the function lives.
/// The component_id is the type of the LazySignalsImmutable where the result will be stored.
pub trait PropagatorContext: Send + Sync + FnMut(&DynamicTuple, &Entity, &mut World) -> bool {}
impl<T: Send + Sync + FnMut(&DynamicTuple, &Entity, &mut World) -> bool> PropagatorContext for T {}

// Let the developer pass in a regular Rust closure that borrows a concrete typed tuple as params.
// The return type is Option<LazySignalsData> which can then be memoized.
pub trait Propagator<P: LazySignalsParams, R: LazySignalsData>: Send +
    Sync +
    'static +
    Fn(P) -> LazySignalsResult<R> {}
impl<
    P: LazySignalsParams,
    R: LazySignalsData,
    T: Send + Sync + 'static + Fn(P) -> LazySignalsResult<R>
> Propagator<P, R> for T {}

pub trait PropagatorSubsFn: Send + Sync + Fn(&ComputedImmutable) -> Vec<Entity> {}
impl<T: Send + Sync + Fn(&ComputedImmutable) -> Vec<Entity>> PropagatorSubsFn for T {}

// TODO provide a to_effect to allow a propagator to be used as an effect?

/// This is the same basic thing but this fn just runs side-effects so no value is returned.
/// However, there is a result so we use the unit type.
pub trait EffectContext: Send + Sync + FnMut(&DynamicTuple, &mut World) {}
impl<T: Send + Sync + FnMut(&DynamicTuple, &mut World)> EffectContext for T {}

// Let the developer pass in a regular Rust closure that borrows a concrete typed tuple as params.
pub trait Effect<P: LazySignalsParams>: Send + Sync + 'static + FnMut(P, &mut World) {}
impl<P: LazySignalsParams, T: Send + Sync + 'static + FnMut(P, &mut World)> Effect<P> for T {}

pub trait EffectSubsFn: Send + Sync + Fn(&LazyEffect) -> Vec<Entity> {}
impl<T: Send + Sync + Fn(&LazyEffect) -> Vec<Entity>> EffectSubsFn for T {}

pub trait ObservableFn: Send +
    Sync +
    FnMut(
        Box<&mut dyn LazySignalsObservable>,
        Option<&mut DynamicTuple>,
        Option<&Entity>
    ) -> MaybeFlaggedEntities {}
impl<
    T: Send +
        Sync +
        FnMut(
            Box<&mut dyn LazySignalsObservable>,
            Option<&mut DynamicTuple>,
            Option<&Entity>
        ) -> MaybeFlaggedEntities
> ObservableFn for T {}

/// ## Component Structs
///
/// An ImmutableState stores the ComponentId of an LazyImmutable<T> with concrete T.
#[derive(Component)]
pub struct ImmutableState {
    pub component_id: ComponentId,
}

/// A SendSignal component marks a LazyImmutable cell as having a next_value.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct SendSignal;

/// A ComputedImmutable is a Propagator that memoizes its result in a LazyImmutable.
#[derive(Component)]
pub struct ComputedImmutable {
    pub function: Box<dyn PropagatorContext>,
    pub sources: Vec<Entity>,
    pub params_type: TypeId,
    pub lazy_immutable_type: TypeId,
}

/// A ComputeMemo component marks a ComputedImmutable that needs computin.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct ComputeMemo;

/// A LazyEffect is a Propagator-like endpoint that returns no value and just runs side-effects.
#[derive(Component)]
pub struct LazyEffect {
    pub function: Box<dyn EffectContext>,
    pub sources: Vec<Entity>,
    pub triggers: Vec<Entity>,
    pub params_type: TypeId,
}

/// A DeferredEffect component marks an LazyEffect function that needs to run.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct DeferredEffect;

/// Marks a ComputedImmutable or LazyEffect as needing to subscribe to its dependencies.
/// This normally only happens within the framework internals on create.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct RebuildSubscribers;
