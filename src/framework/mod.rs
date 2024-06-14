use std::{ any::TypeId, fmt::Debug, sync::Mutex };

use bevy::{
    ecs::{ component::{ ComponentId, ComponentInfo }, storage::SparseSet, system::CommandQueue },
    prelude::*,
    reflect::{ DynamicTuple, GetTypeRegistration, Tuple },
    tasks::Task,
};

use thiserror::Error;

use crate::LazySignalsObservable;

pub mod bundles;
pub mod lazy_immutable;

/// # Signals framework
/// ## Types
/// Result type for handling error conditions in developer code.
pub type LazySignalsResult<R> = Option<Result<R, LazySignalsError>>;

/// Return type for an optional list of entities and some flags (changed, triggered).
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
    #[error("Error reading signal {0:?}")]
    ReadError(Entity),
}

// ## Traits
/// An item of data for use with Immutables.
pub trait LazySignalsData: FromReflect +
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
        T: FromReflect +
            GetTypeRegistration +
            PartialEq +
            Reflect +
            Send +
            Sync +
            TypePath +
            'static {}

/// A tuple containing parameters for a computed memo or effect.
pub trait LazySignalsArgs: LazySignalsData + Tuple {}
impl<T> LazySignalsArgs for T where T: LazySignalsData + Tuple {}

/// A Propagator function aggregates (merges) data from multiple cells to store in a bound cell.
/// Compared to the MIT model, the Computed pulls data into a cell they are bound to.
/// MIT Propagators are conceptually more independent and closer to a push-based flow.
/// This Computed merges the values of cells denoted by the entity vector into the target entity.
///
/// The DynamicTuple is an argument list whose internal types match the Option<T> of each source.
/// (i.e. SignalsResult<T> becomes Option<T> with any Err becoming None)
///
/// The entity is where the result will be stored, where this instance of the function lives.
///
/// The world is the world is love and life are deep.
pub trait ComputedContext: Send + Sync + FnMut(&DynamicTuple, &Entity, &mut World) -> bool {}
impl<T: Send + Sync + FnMut(&DynamicTuple, &Entity, &mut World) -> bool> ComputedContext for T {}

/// Let the developer pass in a regular Rust closure that borrows a concrete typed tuple as args.
/// The return type is a LazySignalsResult which can then be memoized.
pub trait Computed<P: LazySignalsArgs, R: LazySignalsData>: Send +
    Sync +
    'static +
    Fn(P) -> LazySignalsResult<R> {}
impl<
    P: LazySignalsArgs,
    R: LazySignalsData,
    T: Send + Sync + 'static + Fn(P) -> LazySignalsResult<R>
> Computed<P, R> for T {}

/// This is the same basic thing but this fn just runs side-effects so no value is returned.
pub trait EffectWrapper: Send + Sync + FnMut(&DynamicTuple, &mut World) {}
impl<T: Send + Sync + FnMut(&DynamicTuple, &mut World)> EffectWrapper for T {}

/// Let the developer pass in a regular Rust closure that borrows a concrete typed tuple as args.
pub trait Effect<P: LazySignalsArgs>: Send + Sync + 'static + FnMut(P, &mut World) {}
impl<P: LazySignalsArgs, T: Send + Sync + 'static + FnMut(P, &mut World)> Effect<P> for T {}

pub trait TaskWrapper: Send + Sync + Fn(&DynamicTuple) -> Task<CommandQueue> {}
impl<T: Send + Sync + Fn(&DynamicTuple) -> Task<CommandQueue>> TaskWrapper for T {}

pub trait AsyncTask<P: LazySignalsArgs>: Send + Sync + 'static + Fn(P) -> Task<CommandQueue> {}
impl<P: LazySignalsArgs, T: Send + Sync + 'static + Fn(P) -> Task<CommandQueue>> AsyncTask<P>
for T {}

pub enum EffectContext {
    Short(Mutex<Box<dyn EffectWrapper>>),
    Long(Mutex<Box<dyn TaskWrapper>>),
}

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

/// A ComputedImmutable is a Computed that memoizes its result in a LazySignalsState.
#[derive(Component)]
pub struct ComputedImmutable {
    pub function: Mutex<Box<dyn ComputedContext>>,
    pub sources: Vec<Entity>,
    pub args_type: TypeId,
    pub result_type: TypeId,
}

/// A ComputeMemo component marks a Computed function that needs computin.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct ComputeMemo;

/// A LazyEffect returns no value and just runs side-effects.
#[derive(Component)]
pub struct LazyEffect {
    pub function: EffectContext,
    pub sources: Vec<Entity>,
    pub triggers: Vec<Entity>,
    pub args_type: TypeId,
}

/// A DeferredEffect component marks an Effect function that needs to run.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct DeferredEffect;

/// A Dirty component means that a value _may_ have changed and needs to be evaluated.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct Dirty;

/// Marks a ComputedImmutable or LazyEffect as needing to subscribe to its dependencies.
/// This normally only happens within the framework internals on create.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct InitDependencies;

/// A RunningTask component marks an Effect function that may still be running.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct RunningTask {
    pub task: Task<CommandQueue>,
}

/// A Triggered component marks a Computed triggers any effect anywhere down its subscriber tree.
/// It also marks any Effect that has been triggered this way.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct Triggered;

/// A ValueChanged component marks a Signal or Component that actually changed.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct ValueChanged;

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
