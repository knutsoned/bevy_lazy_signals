use bevy_ecs::{ component::{ ComponentId, ComponentInfo }, prelude::*, storage::SparseSet };
use bevy_reflect::{ reflect_trait, Reflect };
use bevy_utils::tracing::*;

use thiserror::Error;

/// # Signals framework
/// ## Enums
/// Read error.
#[derive(Error, Debug)]
pub enum SignalsError {
    #[error("Error reading signal {0}")] ReadError(Entity),
    #[error["Signal does not exist"]] NoSignalError,
}

// ## Traits
/// An item of data backed by a Bevy entity with a set of subscribers.
/// Additional methods in UntypedObservable would be here but you can't have generic trait objects.
pub trait Observable: Send + Sync + 'static {
    type DataType: Copy + PartialEq + Send + Sync + 'static;

    /// Get the current value.
    fn read(&self) -> Self::DataType;

    /// Get the current value, subscribing an entity if provided (mostly used internally).
    fn value(&mut self, caller: Entity) -> Self::DataType;
}

/// These methods support lazy operations. These are part of sending a Signal.
pub trait LazyObservable: Send + Sync + 'static {
    type DataType: Copy + PartialEq + Send + Sync + 'static;

    /// Called by a consumer to provide a new value for the lazy update system to merge.
    fn merge_next(&mut self, next: Self::DataType);
}

#[reflect_trait]
pub trait UntypedObservable {
    /// Called by a lazy update system to apply the new value of a signal.
    /// This is a main thing to implement if you're trying to use reflection.
    /// The ref impl uses this to update the Immutable values without knowing the type.
    /// These are also part of sending a Signal.
    ///
    /// This method returns a vector of subscriber Entities that may need notification.
    fn merge(&mut self) -> Vec<Entity>;

    /// Called by a lazy update system to refresh the subscribers.
    fn merge_subscribers(&mut self);

    /// Called by an Effect or Memo indirectly by reading the current value.
    fn subscribe(&mut self, entity: Entity);
}

/// A Propagator function aggregates (merges) data from multiple cells to store in a bound cell.
/// Compared to the MIT model, these Propagators pull data into a cell they are bound to.
/// MIT Propagators are conceptually more independent and closer to a push-based flow.
/// This Propagator merges the values of cells denoted by the entity vector into the target entity.
/// It should call value instead of read to make sure it is re-subscribed to its sources!
/// If the target entity is not supplied, the function is assumed to execute side effects only.
pub type PropagatorFn = dyn FnMut(&mut World, &mut Vec<Entity>, Option<&mut Entity>) + Send + Sync;

/// ## Component Structs
/// An Immutable is known as a cell in a propagator network. It may also be referred to as state.
/// Using the label Immutable because Cell and State often mean other things.
/// Mutable is used by futures-signals for the same data-wrapping purpose, but in our case, the
/// cells are mutated by sending a signal explicitly (i.e. adding a SendSignal component).
///
/// Some convenience types provided: ImmutableBool, ImmutableInt, ImmutableFloat, ImmutableString.
///
/// The subscriber set is built from the sources/triggers of computed memos and effects, so it does
/// not have to be serialized, which is good because the SparseSet doesn't seem to do Reflect.
///
/// This Immutable is lazy. Other forms are left as an exercise for the reader.
#[derive(Component, Reflect)]
#[reflect(Component, UntypedObservable)]
pub struct LazyImmutable<T: Copy + PartialEq + Send + Sync + 'static> {
    data: T,
    next_value: Option<T>,
    #[reflect(ignore)]
    subscribers: EntitySet,
    #[reflect(ignore)]
    next_subscribers: EntitySet,
}

impl<T: Copy + PartialEq + Send + Sync + 'static> LazyImmutable<T> {
    pub fn new(data: T) -> Self {
        Self { data, next_value: None, subscribers: empty_set(), next_subscribers: empty_set() }
    }
}

impl<T: Copy + PartialEq + Send + Sync + 'static> Observable for LazyImmutable<T> {
    type DataType = T;

    fn read(&self) -> Self::DataType {
        self.data
    }

    fn value(&mut self, caller: Entity) -> Self::DataType {
        self.subscribe(caller);
        self.read()
    }
}

impl<T: Copy + PartialEq + Send + Sync + 'static> UntypedObservable for LazyImmutable<T> {
    fn merge(&mut self) -> Vec<Entity> {
        let mut subs = Vec::<Entity>::new();

        // update the Immutable data value
        if let Some(next) = self.next_value {
            info!("next exists");
            if self.data != next {
                info!("data != next");
                self.data = next;

                // copy the subscribers into the output vector
                subs.extend(self.subscribers.indices());

                // clear the local subscriber set which will be replenished by each subscriber if it calls
                // the value method later
                self.subscribers.clear();
            }
        }
        self.next_value = None;

        subs
    }

    fn merge_subscribers(&mut self) {
        for subscriber in self.next_subscribers.iter() {
            self.subscribers.insert(*subscriber.0, ());
        }
        self.next_subscribers.clear();
    }
    fn subscribe(&mut self, entity: Entity) {
        self.next_subscribers.insert(entity, ());
    }
}

impl<T: Copy + PartialEq + Send + Sync + 'static> LazyObservable for LazyImmutable<T> {
    type DataType = T;

    fn merge_next(&mut self, next: T) {
        self.next_value = Some(next);
    }
}

#[derive(Component)]
pub struct ImmutableComponentId {
    pub component_id: ComponentId,
}

/// A SendSignal component marks an Immutable cell as having a next_value.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct SendSignal;

/// A Propagator component is the aggregating propagator function and its sources/triggers list.
#[derive(Component)]
pub struct Propagator {
    pub propagator: &'static PropagatorFn,
    pub sources: Vec<Entity>,
}

/// A ComputeMemo component marks an Immutable that needs to be computed.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct ComputeMemo;

/// A DeferredEffect component marks a Propagator function that needs to run.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct DeferredEffect;

/// ## Utilities
/// Type alias for SparseSet<Entity, ()>
pub type EntitySet = SparseSet<Entity, ()>;

/// Create an empty sparse set for storing Entities by ID
pub fn empty_set() -> EntitySet {
    EntitySet::new()
}

pub type ComponentIdSet = SparseSet<Entity, ComponentId>;
pub type ComponentInfoSet = SparseSet<ComponentId, ComponentInfo>;

pub type ImmutableBool = LazyImmutable<bool>;
pub type ImmutableInt = LazyImmutable<u32>;
pub type ImmutableFloat = LazyImmutable<f64>;
pub type ImmutableStr = LazyImmutable<&'static str>;
