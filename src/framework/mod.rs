use std::{ any::TypeId, fmt::Debug };

use bevy::{
    ecs::{ component::{ ComponentId, ComponentInfo }, storage::SparseSet },
    prelude::*,
    reflect::{ DynamicTuple, GetTypeRegistration, Tuple },
};

use thiserror::Error;

use crate::{ LazySignalsObservable, LazySignalsState };

pub mod lazy_immutable;

/// # Signals framework
/// ## Types
/// Result type for handling error conditions in developer code.
pub type LazySignalsResult<R> = Option<Result<R, LazySignalsError>>;

/// Return type for returning an optional list of entities and some flags (changed, triggered).
pub type MaybeFlaggedEntities = Option<(Vec<Entity>, bool, bool)>;

/// ## Enums
/// Read error.
#[derive(Error, Clone, Copy, PartialEq, Reflect, Debug)]
pub enum LazySignalsError {
    /// An attempt was made to reference a LazySignals entity that does not exist.
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
pub trait LazySignalsData: Clone +
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
        T: Clone +
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
///
/// The DynamicTuple is an argument list whose internal types match the Option<T> of each source.
/// (i.e. SignalsResult<T> becomes Option<T> with any Err becoming None)
///
/// The entity is where the result will be stored, where this instance of the function lives.
///
/// The world is the world is love and life are deep.
pub trait PropagatorContext: Send + Sync + FnMut(&DynamicTuple, &Entity, &mut World) -> bool {}
impl<T: Send + Sync + FnMut(&DynamicTuple, &Entity, &mut World) -> bool> PropagatorContext for T {}

/// Let the developer pass in a regular Rust closure that borrows a concrete typed tuple as params.
/// The return type is a LazySignalsResult which can then be memoized.
pub trait Propagator<P: LazySignalsParams, R: LazySignalsData>: Send +
    Sync +
    'static +
    Fn(P) -> LazySignalsResult<R> {}
impl<
    P: LazySignalsParams,
    R: LazySignalsData,
    T: Send + Sync + 'static + Fn(P) -> LazySignalsResult<R>
> Propagator<P, R> for T {}

/// This is the same basic thing but this fn just runs side-effects so no value is returned.
pub trait EffectContext: Send + Sync + FnMut(&DynamicTuple, &mut World) {}
impl<T: Send + Sync + FnMut(&DynamicTuple, &mut World)> EffectContext for T {}

/// Let the developer pass in a regular Rust closure that borrows a concrete typed tuple as params.
pub trait Effect<P: LazySignalsParams>: Send + Sync + 'static + FnMut(P, &mut World) {}
impl<P: LazySignalsParams, T: Send + Sync + 'static + FnMut(P, &mut World)> Effect<P> for T {}

/// Catch-all fn signature for LazySignalsObservable operations.
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
/// An ImmutableState stores the ComponentId of a LazySignalsState<T> with concrete T.
#[derive(Component)]
pub struct ImmutableState {
    pub component_id: ComponentId,
}

/// A SendSignal component marks a LazySignalsState cell as having a next_value.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct SendSignal;

/// A ComputedImmutable is a Propagator that memoizes its result in a LazySignalsState.
#[derive(Component)]
pub struct ComputedImmutable {
    pub function: Box<dyn PropagatorContext>,
    pub sources: Vec<Entity>,
    pub params_type: TypeId,
    pub result_type: TypeId,
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
pub struct InitDependencies;

/// ## Bundles
#[derive(Bundle)]
pub struct ComputedBundle<R: LazySignalsData> {
    state: LazySignalsState<R>,
    meta: ImmutableState,
    context: ComputedImmutable,
    init: InitDependencies,
}

impl<R: LazySignalsData> ComputedBundle<R> {
    pub fn from_function<P: LazySignalsParams>(
        function: Box<dyn PropagatorContext>,
        sources: Vec<Entity>,
        component_id: ComponentId
    ) -> ComputedBundle<R> {
        ComputedBundle::<R> {
            state: LazySignalsState::<R>::new(None),
            meta: ImmutableState { component_id },
            context: ComputedImmutable {
                function,
                sources,
                params_type: TypeId::of::<P>(),
                result_type: TypeId::of::<LazySignalsState<R>>(),
            },
            init: InitDependencies,
        }
    }
}

#[derive(Bundle)]
pub struct EffectBundle {
    context: LazyEffect,
    init: InitDependencies,
}

impl EffectBundle {
    pub fn from_function<P: LazySignalsParams>(
        function: Box<dyn EffectContext>,
        sources: Vec<Entity>,
        triggers: Vec<Entity>
    ) -> EffectBundle {
        EffectBundle {
            context: LazyEffect {
                function,
                sources,
                triggers,
                params_type: TypeId::of::<P>(),
            },
            init: InitDependencies,
        }
    }
}

#[derive(Bundle)]
pub struct StateBundle<T: LazySignalsData> {
    state: LazySignalsState<T>,
    meta: ImmutableState,
}

impl<T: LazySignalsData> StateBundle<T> {
    pub fn from_value(data: T, component_id: ComponentId) -> StateBundle<T> {
        StateBundle {
            state: LazySignalsState::<T>::new(Some(Ok(data))),
            meta: ImmutableState { component_id },
        }
    }
}

/// ## Utilities
/// Set of Entity to ComponentId.
pub type ComponentIdSet = SparseSet<Entity, ComponentId>;

/// Set of ComponentId to ComponentInfo.
pub type ComponentInfoSet = SparseSet<ComponentId, ComponentInfo>;

/// Set of Entity to child Entities.
pub type EntityRelationshipSet = SparseSet<Entity, Vec<Entity>>;

/// Set of unique Entities
pub type EntitySet = SparseSet<Entity, ()>;

/// Set of internal errors when running computed (propagator) and effect functions.
pub type ErrorSet = SparseSet<Entity, LazySignalsError>;

/// Create an empty sparse set for storing Entities by ID.
pub fn empty_set() -> EntitySet {
    EntitySet::new()
}
