use bevy_ecs::prelude::*;

use crate::utilities::*;

/// # Signals framework
///
/// ## Traits
/// A user API patterned after the [TC39 proposal](https://github.com/tc39/proposal-signals)
/// up to and including _Introducing Signals_. We differ by adding explicit read and send instead
/// of get and set. Semantically, get and set perform extra work in the proposed implementation.
/// Our implementation propagates all values during processing instead of waiting for a read.
/// We do more work during processing but less work during the read and send operations.
/// This may be desirable since it keeps the necessity to obtain exclusive world write access to a
/// minimum in the User Code.
pub trait Signal {
    /// Create a new computed entity (Immutable + Propagator).
    fn computed<T: Copy + Default + PartialEq + Send + Sync + 'static>(
        propagator: Box<dyn PropagatorFn>,
        sources: Vec<Entity>,
        commands: Commands
    ) -> Entity;

    /// Create a new effect entity (Propagator).
    fn effect(
        propagator: Box<dyn PropagatorFn>,
        triggers: Vec<Entity>,
        commands: Commands
    ) -> Entity;

    /// Read an Immutable's value without subscribing to it.
    /// Typically an effect would be triggered instead of using this method.
    /// However, if using an immediate mode UI or reading additional values while running an effect
    /// (saving to a file or sending over a network) then it may be useful to read values directly.
    fn read<T: Copy + Default + PartialEq + Send + Sync + 'static>(
        immutable: Entity,
        world: &World
    ) -> T;

    /// Mark an Immutable (Signal) for update.
    fn send<T: Copy + Default + PartialEq + Send + Sync + 'static>(
        signal: Entity,
        data: T,
        commands: Commands
    );

    /// Create a new Immutable state entity (Signal).
    fn state<T: Copy + Default + PartialEq + Send + Sync + 'static>(
        value: T,
        commands: Commands
    ) -> Entity;
}

/// An item of data backed by a Bevy entity with a set of subscribers.
pub trait Observable: Send + Sync + 'static {
    type DataType: Copy + Default + PartialEq + Send + Sync + 'static;

    /// Called by an Effect or Memo indirectly by reading the current value.
    fn subscribe(&mut self, entity: Entity);

    /// Get the current value.
    fn read(&self) -> Self::DataType;

    /// Get the current value, subscribing an entity if provided (mostly used internally).
    fn value(&mut self, caller: Entity) -> Self::DataType;
}

pub trait LazyObservable: Send + Sync + 'static {
    type DataType: Copy + Default + PartialEq + Send + Sync + 'static;

    /// Called by a lazy update system to apply the new value of a signal.
    fn merge(&mut self);

    /// Called by a consumer to provide an new value for the lazy update system to merge.
    fn merge_next(&mut self, next: Self::DataType);

    /// Called by a lazy update system to refresh the subscribers.
    fn merge_subscribers(&mut self);
}

/// A propagator function aggregates (merges) data from multiple cells for storage in a bound cell.
/// Compared to the MIT model, these propagators pull data into a cell they are bound to.
/// MIT propagators are conceptually more independent and closer to a push-based flow.
/// The propagator merges the values of cells denoted by the entity vector into the target entity.
/// It should call value instead of read to make sure it is re-subscribed to its sources!
/// If the target entity is not supplied, the function is assumed to execute side effects only.
pub trait PropagatorFn: Send + Sync + FnMut(&mut World, &mut Vec<Entity>, Option<&mut Entity>) {}

/// ## Component Structs
/// An immutable is known as a cell in a propagator network. It may also be referred to as state.
/// Using the label Immutable because Cell and State often mean other things.
/// Mutable is used by futures-signals for the same data-wrapping purpose, but in our case, the
/// cells are mutated by sending a signal explicitly (i.e. adding a Signal component).
///
/// This Immutable is lazy. Other forms are left as an exercise for the reader.
#[derive(Component)]
pub struct LazyImmutable<T: Copy + Default + PartialEq + Send + Sync + 'static> {
    data: T,
    next_value: Option<T>,
    subscribers: EntitySet,
    next_subscribers: EntitySet,
}

impl<T: Copy + Default + PartialEq + Send + Sync + 'static> LazyImmutable<T> {
    pub fn new(data: T) -> Self {
        Self { data, next_value: None, subscribers: empty_set(), next_subscribers: empty_set() }
    }
}

impl<T: Copy + Default + PartialEq + Send + Sync + 'static> Observable for LazyImmutable<T> {
    type DataType = T;

    fn read(&self) -> Self::DataType {
        self.data
    }

    fn subscribe(&mut self, entity: Entity) {
        self.next_subscribers.insert(entity, ());
    }

    fn value(&mut self, caller: Entity) -> Self::DataType {
        self.subscribe(caller);
        self.read()
    }
}

impl<T: Copy + Default + PartialEq + Send + Sync + 'static> LazyObservable for LazyImmutable<T> {
    type DataType = T;

    fn merge(&mut self) {
        if let Some(next) = self.next_value {
            self.data = next;
        }
        self.next_value = None;
    }

    fn merge_next(&mut self, next: T) {
        self.next_value = Some(next);
    }

    fn merge_subscribers(&mut self) {
        for subscriber in self.next_subscribers.iter() {
            self.subscribers.insert(*subscriber.0, ());
        }
        self.next_subscribers.clear();
    }
}

/// A SendSignal component marks an Immutable cell as having a next_value.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct SendSignal;

/// A Propagator component is the aggregating propagator function and its paramater list.
#[derive(Component)]
pub struct Propagator {
    pub propagator: Box<dyn PropagatorFn>,
    pub sources: Vec<Entity>,
}

/// A ComputeMemo component marks an Immutable that needs to be computed.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct ComputeMemo;

/// An Effect component marks a Propagator function that needs to run.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct DeferredEffect;
