use std::{ any::TypeId, fmt::Debug, marker::PhantomData, sync::Mutex };

use bevy::{
    ecs::{
        component::{ ComponentId, ComponentInfo },
        storage::SparseSet,
        system::BoxedSystem,
        world::CommandQueue,
    },
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
#[derive(PartialEq, Reflect)]
pub struct LazySignalsResult<R: LazySignalsData> {
    pub data: Option<R>,
    pub error: Option<LazySignalsError>,
}

/// Return type for an optional list of entities and some flags (changed, triggered).
pub type MaybeFlaggedEntities = Option<(LazySignalsVec, bool, bool)>;

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

pub trait LazySignalsSources<P: LazySignalsArgs>: LazySignalsData + Tuple {}
impl<P: LazySignalsArgs, T> LazySignalsSources<P> for T where T: LazySignalsData + Tuple {}

impl<P: LazySignalsArgs> From<P> for LazySignalsVec {
    fn from(value: P) -> Self {
        todo!()
    }
}

// #[derive(Copy)]
pub struct LazySignalsVec(pub Vec<Entity>);

impl LazySignalsVec {
    pub fn new() -> Self {
        Self(Vec::<Entity>::new())
    }

    pub fn append(&mut self, other: &mut LazySignalsVec) {
        self.0.append(&mut other.0);
    }
}

impl Clone for LazySignalsVec {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl Debug for LazySignalsVec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl From<LazySignalsVec> for Vec<Entity> {
    fn from(value: LazySignalsVec) -> Self {
        value.0
    }
}

impl IntoIterator for LazySignalsVec {
    type Item = Entity;

    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

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

/// This is the same basic thing but the fn just runs side-effects so it may return a system to run.
pub trait EffectWrapper: Send + Sync + FnMut(&DynamicTuple, &mut World) -> Option<BoxedSystem> {}
impl<T: Send + Sync + FnMut(&DynamicTuple, &mut World) -> Option<BoxedSystem>> EffectWrapper
for T {}

/// Let the developer pass in a regular Rust closure that borrows a concrete typed tuple as args.
pub trait Effect<P: LazySignalsArgs>: Send +
    Sync +
    'static +
    FnMut(P, &mut World) -> Option<BoxedSystem> {}
impl<
    P: LazySignalsArgs,
    T: Send + Sync + 'static + FnMut(P, &mut World) -> Option<BoxedSystem>
> Effect<P> for T {}

pub trait ActionWrapper: Send + Sync + Fn(&DynamicTuple) -> Task<CommandQueue> {}
impl<T: Send + Sync + Fn(&DynamicTuple) -> Task<CommandQueue>> ActionWrapper for T {}

pub trait Action<P: LazySignalsArgs>: Send + Sync + 'static + Fn(P) -> Task<CommandQueue> {}
impl<P: LazySignalsArgs, T: Send + Sync + 'static + Fn(P) -> Task<CommandQueue>> Action<P> for T {}

pub enum EffectContext {
    Short(Mutex<Box<dyn EffectWrapper>>),
    Long(Mutex<Box<dyn ActionWrapper>>),
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

/// Wrap an input to a Computed or Effect as an entity with a LazySignalsState of a certain type.
/// This would generally be obtained from an API factory object.
pub struct Src<T: LazySignalsData> {
    pub entity: Entity,
    arg_type: PhantomData<T>,
}

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
    pub sources: LazySignalsVec,
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
    pub sources: LazySignalsVec,
    pub triggers: LazySignalsVec,
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
pub type EntityRelationshipSet = SparseSet<Entity, LazySignalsVec>;

/// Set of unique Entities
pub type EntitySet = SparseSet<Entity, ()>;

/// Set of internal errors when running computed (propagator) and effect functions.
pub type ErrorSet = SparseSet<Entity, LazySignalsError>;

/// Create an empty sparse set for storing Entities by ID.
pub fn empty_set() -> EntitySet {
    EntitySet::new()
}
