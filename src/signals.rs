use bevy_ecs::{ prelude::*, storage::SparseSet };

/// # Signals framework
///
/// ## Overview
///
/// An Immutable component holds the Observable value. To send a signal, add a Signal component
/// containing the next value. Add a Propagator component to form a memo. A Propagator component
/// without an Immutable is an Effect.
///
/// The Signal component may be set at any time outside of processing. During processing, a (brief)
/// write lock for the world is obtained. Each
///
/// ## Traits
/// An item of data backed by a Bevy entity with a set of subscribers.
pub trait Observable: Send + Sync + 'static {
    type DataType: Copy + PartialEq + Send + Sync + 'static;
    fn merge(&mut self);
    fn send_signal(&mut self, next: Self::DataType);
    fn subscribe(&mut self, entity: Entity);
    fn value(&mut self, caller: Option<Entity>) -> Self::DataType;
}

/// A propagator function aggregates (merges) data from multiple cells for storage in a bound cell.
/// Compared to the MIT model, these propagators pull data into a cell they are bound to.
/// MIT propagators are conceptually more independent and closer to a push-based flow.
/// The propagator merges the cells specified by the entity vector.
pub trait PropagatorFn: Send + Sync + FnMut(&mut World, &mut Vec<Entity>) {}

/// ## Structs
/// ### Components
/// An immutable is known as a cell in a propagator network. It may also be referred to as state.
/// Using the label Immutable because Cell and State often mean other things.
/// Mutable is used by futures-signals for the same data-wrapping purpose, but in our case, the
/// cells are mutated by sending a signal explicitly (i.e. adding a Signal component).
#[derive(Component)]
pub struct Immutable<T: Copy + PartialEq + Send + Sync + 'static> {
    pub data: T,
    next_value: Option<T>,
    subscribers: SparseSet<Entity, ()>,
}

impl<T: Copy + PartialEq + Send + Sync + 'static> Immutable<T> {
    pub fn new(data: T) -> Self {
        let subscribers = SparseSet::<Entity, ()>::new();
        Self { data, next_value: None, subscribers }
    }
}

impl<T: Copy + PartialEq + Send + Sync + 'static> Observable for Immutable<T> {
    type DataType = T;

    fn merge(&mut self) {
        if let Some(next) = self.next_value {
            self.data = next;
        }
    }

    fn send_signal(&mut self, next: T) {
        self.next_value = Some(next);
    }

    fn subscribe(&mut self, entity: Entity) {
        self.subscribers.insert(entity, ());
    }

    fn value(&mut self, caller: Option<Entity>) -> Self::DataType {
        if let Some(caller) = caller {
            self.subscribe(caller);
        }
        self.data
    }
}

/// A Signal component marks an Immutable cell as having a next_value.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct Signal {}

/// A Propagator component is the aggregating propagator function and its paramater list.
#[derive(Component)]
pub struct Propagator {
    pub propagator: Box<dyn PropagatorFn>,
    pub sources: Vec<Entity>,
}

/// A Memo component marks an Immutable that needs to be computed.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct Memo {}

/// An Effect component marks a Propagator function that needs to run.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct Effect {}

/// ## Systems
/// These systems are meant to be run in tight sequence, preferably as a chain
pub fn signals_system(query_signals: Query<Entity, With<Signal>>, world: &mut World) {
    // Phase One:

    // apply the next value to each Immutable

    // add subscribers to the running set

    // clear subscribers from the current Immutable

    // remove the Signal component

    // Phase Two:

    // iterate through a copy of the running set

    // remove an item from the running set

    // skip if already in handled set

    // add the item to the handled set

    // a) item is an effect, so schedule the effect by adding an Effect component

    // b1) item is a memo, so mark it for recalculation

    // b2) item has its own subscribers, so add those to the running set

    // loop through the running set until it is empty

}

pub fn memos_system(query_memos: Query<Entity, With<Memo>>, world: &mut World) {
    // run the Propagator function to recalculate memo

    // update the data in the cell

    // remove the Memo component
}

pub fn effects_system(
    query_effects: Query<Entity, With<Effect>>,
    commands: &mut Commands,
    world: &World
) {
    // spawn a thread for each effect

    // remove the Effect component
}
