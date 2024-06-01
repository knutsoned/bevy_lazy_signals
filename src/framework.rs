use std::any::TypeId;

use bevy::{
    ecs::{ component::{ ComponentId, ComponentInfo }, storage::SparseSet },
    prelude::*,
    reflect::{ DynamicTuple, GetTypeRegistration, Tuple },
};

use thiserror::Error;

/// # Signals framework
/// ## Types
/// could not get &String to work
pub type LazySignalsStr = &'static str;

/// Convenience typedefs.
pub type LazyImmutableBool = LazyImmutable<bool>;
pub type LazyImmutableInt = LazyImmutable<u32>;
pub type LazyImmutableFloat = LazyImmutable<f64>;
pub type LazyImmutableStr = LazyImmutable<LazySignalsStr>;
pub type LazyImmutableUnit = LazyImmutable<()>; // triggers

/// Result type for handling error conditions in consumer code.
pub type LazySignalsResult<R> = Option<Result<R, LazySignalsError>>;

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

/// An item of data backed by a Bevy entity with a set of subscribers.
/// Additional methods in LazySignalsObservable would be here but you can't have generic trait objects.
pub trait LazySignalsImmutable: Send + Sync + 'static {
    type DataType: LazySignalsData;

    // TODO add a get that returns a result after safely calling read

    // TODO add a get_value that returns a result after safely calling value

    /// Called by a developer to provide a new value for the lazy update system to merge.
    fn merge_next(&mut self, next: LazySignalsResult<Self::DataType>, trigger: bool);

    /// Get the current value.
    fn read(&self) -> LazySignalsResult<Self::DataType>;

    /// Immediately update a new value without triggering any subscribers (mostly used internally).
    fn update(&mut self, next: LazySignalsResult<Self::DataType>);

    /// Get the current value, subscribing an entity if provided (mostly used internally).
    fn value(&mut self, caller: Entity) -> LazySignalsResult<Self::DataType>;
}

#[reflect_trait]
pub trait LazySignalsObservable {
    /// Called by a lazy update system to apply the new value of a signal.
    /// This is a main thing to implement if you're trying to use reflection.
    /// The ref impl uses this to update the LazySignalsImmutable values without knowing the type.
    /// These are also part of sending a Signal.
    ///
    /// Add None to the params.
    fn append_none(&mut self, params: &mut DynamicTuple);

    /// Copy the data into a dynamic tuple of params for the Effect or Propagator to consume.
    fn copy_data(&mut self, caller: Entity, params: &mut DynamicTuple);

    /// Get the list of subscriber Entities that may need notification.
    fn get_subscribers(&self) -> Vec<Entity>;

    /// Is this signal being forced to trigger?
    fn is_triggered(&self) -> bool;

    /// This method merges the next_value and returns get_subscribers().
    fn merge(&mut self) -> Vec<Entity>;

    /// Called by a lazy update system to refresh the subscribers.
    fn merge_subscribers(&mut self);

    /// Called by an Effect or Memo indirectly by reading the current value.
    fn subscribe(&mut self, entity: Entity);

    /// Called to force a subscriber to a triggered signal to also trigger.
    fn trigger(&mut self);
}

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
pub trait PropagatorContext: Send + Sync + FnMut(&DynamicTuple, &Entity, &mut World) {}
impl<T: Send + Sync + FnMut(&DynamicTuple, &Entity, &mut World)> PropagatorContext for T {}

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

// TODO provide a to_effect to allow a propagator to be used as an effect?

/// This is the same basic thing but this fn just runs side-effects so no value is returned.
/// However, there is a result so we use the unit type.
pub trait EffectContext: Send + Sync + FnMut(&DynamicTuple, &mut World) {}
impl<T: Send + Sync + FnMut(&DynamicTuple, &mut World)> EffectContext for T {}

// Let the developer pass in a regular Rust closure that borrows a concrete typed tuple as params.
pub trait Effect<P: LazySignalsParams>: Send +
    Sync +
    'static +
    FnMut(P, &mut World) -> LazySignalsResult<()> {}
impl<
    P: LazySignalsParams,
    T: Send + Sync + 'static + FnMut(P, &mut World) -> LazySignalsResult<()>
> Effect<P> for T {}

/// ## Component Structs
/// A LazyImmutable is known as a cell in a propagator network. It may also be referred to as state.
/// Using the label LazyImmutable because Cell and State often mean other things.
/// Mutable is used by futures-signals for the same data-wrapping purpose, but in our case, the
/// cells are mutated by sending a signal explicitly
/// (i.e. calling merge_next and adding a SendSignal component).
///
/// Some convenience types provided: ImmutableBool, ImmutableInt, ImmutableFloat, ImmutableStr.
///
/// The subscriber set is built from the sources/triggers of computed memos and effects, so it does
/// not have to be serialized, which is good because the SparseSet doesn't seem to do Reflect.
///
/// This LazyImmutable component is lazy. Other forms are left as an exercise for the reader.
#[derive(Component, Reflect)]
#[reflect(Component, LazySignalsObservable)]
pub struct LazyImmutable<T: LazySignalsData> {
    data: LazySignalsResult<T>,
    next_value: LazySignalsResult<T>,
    triggered: bool,
    #[reflect(ignore)]
    subscribers: EntitySet,
    #[reflect(ignore)]
    next_subscribers: EntitySet,
}

impl<T: LazySignalsData> LazyImmutable<T> {
    pub fn new(data: LazySignalsResult<T>) -> Self {
        Self {
            data,
            next_value: Some(Err(LazySignalsError::NoNextValue)),
            triggered: false,
            subscribers: empty_set(),
            next_subscribers: empty_set(),
        }
    }
}

impl<T: LazySignalsData> LazySignalsImmutable for LazyImmutable<T> {
    type DataType = T;

    fn merge_next(&mut self, next_value: LazySignalsResult<T>, triggered: bool) {
        self.next_value = next_value;
        self.triggered = triggered;
    }

    fn read(&self) -> LazySignalsResult<Self::DataType> {
        self.data
    }

    fn value(&mut self, caller: Entity) -> LazySignalsResult<Self::DataType> {
        self.subscribe(caller);
        self.read()
    }

    fn update(&mut self, next: LazySignalsResult<Self::DataType>) {
        self.data = next;
    }
}

impl<T: LazySignalsData> LazySignalsObservable for LazyImmutable<T> {
    fn append_none(&mut self, params: &mut DynamicTuple) {
        params.insert::<Option<T>>(None);
    }

    fn copy_data(&mut self, caller: Entity, params: &mut DynamicTuple) {
        let data = match self.data {
            Some(data) =>
                match data {
                    Ok(data) => {
                        info!("--inserted data into params");
                        Some(data)
                    }

                    // FIXME do something else with the error
                    Err(error) => {
                        error!("--error: {:?}", error);
                        None
                    }
                }
            None => {
                info!("--no data");
                None
            }
        };
        params.insert(data);

        self.subscribe(caller);
    }

    fn get_subscribers(&self) -> Vec<Entity> {
        let mut subs = Vec::<Entity>::new();

        // copy the subscribers into the output vector
        subs.extend(self.subscribers.indices());
        info!("-found subs {:?}", self.subscribers);
        subs
    }

    fn is_triggered(&self) -> bool {
        self.triggered
    }

    fn merge(&mut self) -> Vec<Entity> {
        // whether or not to overwrite the existing data
        let mut doo_eet = self.triggered;

        // output vector for downstream subscribers to process next
        let mut subs = Vec::<Entity>::new();

        // update the Immutable data value
        match self.next_value {
            Some(Ok(next)) => {
                trace!("next exists");

                // only fire the rest of the process if the data actually changed
                if let Some(Ok(data)) = self.data {
                    trace!("data exists");

                    if data != next {
                        info!("data != next");
                        doo_eet = true;
                    }
                } else {
                    // if data is not a value and not NoValue, always replace
                    doo_eet = true;
                }
            }
            Some(Err(LazySignalsError::NoNextValue)) => {
                // don't clobber the data with a null placeholder (different from None)
                doo_eet = false;
            }
            Some(Err(_)) => {
                // do merge any actual upstream errors
                doo_eet = true;
            }
            None => {
                // None is a valid value for a state, so clobber away
                doo_eet = true;
            }
        }

        // overwrite the value
        if doo_eet {
            self.data = self.next_value;
            self.next_value = Some(Err(LazySignalsError::NoNextValue));
        }

        // return a list of subscribers
        if doo_eet || self.triggered {
            // copy the subscribers into the output vector
            subs = self.get_subscribers();

            // clear the local subscriber set which will be replenished by each subscriber if
            // it calls the value method later
            self.subscribers.clear();

            // trigger is processed, so reset the flag
            self.triggered = false;
        }
        subs
    }

    fn merge_subscribers(&mut self) {
        for subscriber in self.next_subscribers.indices() {
            self.subscribers.insert(subscriber, ());
        }
        self.next_subscribers.clear();
    }

    fn subscribe(&mut self, entity: Entity) {
        self.next_subscribers.insert(entity, ());
    }

    fn trigger(&mut self) {
        self.triggered = true;
    }
}

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
    pub dirty: bool,
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

/// ## Utilities

/// Set of Entity to ComponentId.
pub type ComponentIdSet = SparseSet<Entity, ComponentId>;

/// Set of ComponentId to ComponentInfo.
pub type ComponentInfoSet = SparseSet<ComponentId, ComponentInfo>;

/// Set of Entity to child Entities.
pub type EntityHierarchySet = SparseSet<Entity, Vec<Entity>>;

/// Set of unique Entities
pub type EntitySet = SparseSet<Entity, ()>;

/// Set of internal errors when running computed (propagator) and effect functions.
pub type ErrorSet = SparseSet<Entity, LazySignalsError>;

/// Create an empty sparse set for storing Entities by ID.
pub fn empty_set() -> EntitySet {
    EntitySet::new()
}
