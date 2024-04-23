use bevy_ecs::{ prelude::*, storage::SparseSet };

/// # Signals framework
///
/// ## Overview
/// First, a distinction between semantic structures (Signal, Memo, Effect) and component types
/// (Immutable, SendSignal, ComputeMemo, DeferredEffect): The semantic structures are central
/// concepts but do not exist as a concrete type. The Signal API provides convenience functions
/// to create an ECS entity for each semantic structure that contains the proper Components. The
/// three mappings of valid combinations of components to semantic structures is enumerated in the
/// next sections.
///
/// A Propagator aggregates data from its dependencies. For a Memo, the data is merged into its
/// Immutable. For an Effect, the Propagator uses the dependencies to perform some side effect.
///
/// An Immutable component holds the Observable value. To send a signal, set the next_value and add
/// a SendSignal component. To form a Memo, add a Propagator component to the Immutable. A
/// Propagator component without an Immutable is an Effect. The running set may be a stack in other
/// implementations.
///
/// ### Signal Processing
/// During processing, a (brief) write lock for the world is obtained. A Signal is an Immutable
/// with no Propagator on the same entity. There is no Signal type per se. If the value is
/// unchanged, the SendSignal is discarded. Otherwise, each Signal's data field is replaced with
/// next_value. The Signal is added to a changed set. Subscribers are added to a running set and
/// removed from the Immutable subscribers. Finally the SendSignal component is removed.
///
/// The initial running set is iterated. If the item is a Memo (Propagator with Immutable), then
/// add a ComputeMemo component to mark it for update. If it is an Effect (Propagator without
/// Immutable), add a DeferredEffect component to mark it for scheduling. Walk the subscriber tree,
/// adding each item's subscribers to a new running set and removing them from its own subscribers.
/// As each item is processed, add it to a completed set and do not add any item to a new running
/// set if it exists in the completed set. When the current running set is exhausted, run the new
/// one. The system exits when each item in each running set finishes. This system should be a
/// while loop and not recursive.
///
/// ### Memo Processing
/// The Propagator of every entity marked with a ComputeMemo component runs and the result is
/// stored in the Immutable. As each value is read, the Memo is added to the next_subscribers of
/// the value. If the value is itself a Memo, it will recompute if its ComputeMemo component is
/// present. Otherwise it simply returns its value. If the value is different, it will be added
/// to the changed set which will be used to limit which effects are scheduled.
///
/// This is the only part of the system that is recursive. As each member of the running set will
/// have its own stack when computation begins, it should be resistant to overflows.
///
/// Each element in the running set is added to an executed set and removed from the running set.
/// Any Immutable in the executed set will have its next_subscribers merged into its empty
/// subscribers set at the end of this system. The system exits when each item in the running set
/// finishes.
///
/// ### Effect Processing
/// The effects system compares the dependencies for each entity with a DeferredEffect component
/// against the changed set. If any dependency of an Effect is changed, the Propagator function is
/// called.
///
/// ## Traits
/// A user API patterned after the [TC39 proposal](https://github.com/tc39/proposal-signals)
/// up to and including _Introducing Signals_.
pub trait Signal {
    /// Return a new computed entity (Immutable + Propagator)
    fn computed(
        propagator: Box<dyn PropagatorFn>,
        sources: Vec<Entity>,
        world: &mut World
    ) -> Entity;

    /// Return a new effect entity (Propagator)
    fn effect(
        propagator: Box<dyn PropagatorFn>,
        triggers: Vec<Entity>,
        world: &mut World
    ) -> Entity;

    // Mark an Immutable (Signal) for update
    fn send<T>(next_value: T, world: &mut World);

    /// Return a new Immutable state entity (Signal)
    fn state<T>(value: T, world: &mut World) -> Entity;

    /// Return an Immutable's value
    fn value<T>(immutable: Entity, world: &mut World) -> T;
}

/// An item of data backed by a Bevy entity with a set of subscribers.
pub trait Observable: Send + Sync + 'static {
    type DataType: Copy + PartialEq + Send + Sync + 'static;
    fn merge(&mut self);
    fn merge_subscribers(&mut self);
    fn subscribe(&mut self, entity: Entity);
    fn value(&mut self, caller: Option<Entity>) -> Self::DataType;
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

    fn value(&mut self, caller: Option<Entity>) -> Self::DataType {
        if let Some(caller) = caller {
            self.subscribe(caller);
        }
        self.data
    }
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
