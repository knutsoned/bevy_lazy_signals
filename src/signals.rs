use bevy_ecs::{ prelude::*, storage::SparseSet, world::Command };

/// # Signals framework
///
/// ## Traits
/// A user API patterned after the [TC39 proposal](https://github.com/tc39/proposal-signals)
/// up to and including _Introducing Signals_.
pub trait Signal {
    /// Create a new computed entity (Immutable + Propagator)
    fn computed(
        propagator: Box<dyn PropagatorFn>,
        sources: Vec<Entity>,
        commands: Commands
    ) -> Entity;

    /// Create a new effect entity (Propagator)
    fn effect(
        propagator: Box<dyn PropagatorFn>,
        triggers: Vec<Entity>,
        commands: Commands
    ) -> Entity;

    /// Mark an Immutable (Signal) for update
    fn send<T>(signal: Entity, next_value: T, commands: Commands);

    /// Create a new Immutable state entity (Signal)
    fn state<T>(value: T, commands: Commands) -> Entity;

    /// Borrow an Immutable's value
    fn value<T>(immutable: Entity, commands: Commands) -> T;
}

/// An item of data backed by a Bevy entity with a set of subscribers.
pub trait Observable: Send + Sync + 'static {
    type DataType: Copy + PartialEq + Send + Sync + 'static;

    /// Called by an update system to apply the new value of a signal
    fn merge(&mut self);

    /// Called by an update system to refresh the subscribers.
    fn merge_subscribers(&mut self);

    /// Called by an Effect or Memo indirectly by reading the current value.
    fn subscribe(&mut self, entity: Entity);

    /// Get the current value.
    fn read(&self) -> Self::DataType;

    /// Get the current value, subscribing an entity if provided (mostly used internally).
    fn value(&mut self, caller: Entity) -> Self::DataType;
}

/// A propagator function aggregates (merges) data from multiple cells for storage in a bound cell.
/// Compared to the MIT model, these propagators pull data into a cell they are bound to.
/// MIT propagators are conceptually more independent and closer to a push-based flow.
/// The propagator merges the values of cells denoted by the entity vector into the target entity.
/// If the target entity is not supplied, the function is assumed execute side effects only.
pub trait PropagatorFn: Send + Sync + FnMut(&mut World, Option<&mut Entity>, &mut Vec<Entity>) {}

/// ## Component Structs
/// An immutable is known as a cell in a propagator network. It may also be referred to as state.
/// Using the label Immutable because Cell and State often mean other things.
/// Mutable is used by futures-signals for the same data-wrapping purpose, but in our case, the
/// cells are mutated by sending a signal explicitly (i.e. adding a Signal component).
#[derive(Component)]
pub struct Immutable<T: Copy + PartialEq + Send + Sync + 'static> {
    data: T,
    next_value: Option<T>,
    subscribers: SparseSet<Entity, ()>,
    next_subscribers: SparseSet<Entity, ()>,
}

impl<T: Copy + PartialEq + Send + Sync + 'static> Immutable<T> {
    pub fn new(data: T) -> Self {
        Self { data, next_value: None, subscribers: empty_set(), next_subscribers: empty_set() }
    }

    pub fn merge_next(&mut self, next: T) {
        self.next_value = Some(next);
    }
}

impl<T: Copy + PartialEq + Send + Sync + 'static> Observable for Immutable<T> {
    type DataType = T;

    fn merge(&mut self) {
        if let Some(next) = self.next_value {
            self.data = next;
        }
        self.next_value = None;
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

    fn read(&self) -> Self::DataType {
        self.data
    }

    fn value(&mut self, caller: Entity) -> Self::DataType {
        self.subscribe(caller);
        self.read()
    }
}

struct SendSignalCommand<T: Copy + PartialEq + Send + Sync + 'static> {
    signal: Entity,
    data: T,
}

impl<T: Copy + PartialEq + Send + Sync + 'static> Command for SendSignalCommand<T> {
    fn apply(self, world: &mut World) {}
}

/// A SendSignal component marks an Immutable cell as having a next_value.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct SendSignal {}

/// A Propagator component is the aggregating propagator function and its paramater list.
#[derive(Component)]
pub struct Propagator {
    pub propagator: Box<dyn PropagatorFn>,
    pub sources: Vec<Entity>,
}

/// A ComputeMemo component marks an Immutable that needs to be computed.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct ComputeMemo {}

/// An Effect component marks a Propagator function that needs to run.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct DeferredEffect {}

/// ## Utilities
/// Create an empty sparse set for storing Entities by ID
fn empty_set() -> SparseSet<Entity, ()> {
    SparseSet::<Entity, ()>::new()
}
