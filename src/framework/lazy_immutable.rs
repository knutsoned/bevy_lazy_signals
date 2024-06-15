use bevy::{ prelude::*, reflect::{ reflect_trait, DynamicTuple, Reflect } };

use crate::arcane_wizardry::{ clone_data, insert_data };

use super::*;

/// LazySignalsImmutable is the typed part of the main trait, LazySignalsObservable is the untyped
/// part, and LazySignalsState is the component struct.
///
/// A LazySignalsImmutable is an item of data backed by a Bevy entity with a set of subscribers.
/// Additional methods in LazySignalsObservable would be here but you can't have generic trait
/// objects.
pub trait LazySignalsImmutable: Send + Sync + 'static {
    type DataType: LazySignalsData;

    /// Provide a new value for the lazy update system to merge.
    fn merge_next(&mut self, next: LazySignalsResult<Self::DataType>, trigger: bool);

    /// Immediately update a new value without triggering any subscribers (mostly used internally).
    fn update(&mut self, next: LazySignalsResult<Self::DataType>) -> bool;

    /// Called by a developer to get the current error.
    fn error(&self) -> Option<LazySignalsError>;

    /// Called by a developer to get the current value.
    fn get(&self) -> Option<Self::DataType>;
}

/// Called by a lazy update system to apply the new value of a signal, run effects, etc.
/// This is a main thing to implement if you're trying to use reflection.
/// The ref impl uses this to update the LazySignalsImmutable values without knowing the type.
/// These are also part of sending a Signal.
#[reflect_trait]
pub trait LazySignalsObservable {
    /// Add None to the args.
    fn append_none(&mut self, args: &mut DynamicTuple);

    /// Copy the data into a dynamic tuple of args for the Effect or Propagator to consume.
    fn copy_data(&mut self, caller: Entity, args: &mut DynamicTuple);

    /// Get the list of subscriber Entities that may need notification.
    fn get_subscribers(&self) -> Vec<Entity>;

    /// This method merges the next_value and returns get_subscribers().
    fn merge(&mut self) -> MaybeFlaggedEntities;

    /// Called by a lazy update system to refresh the subscribers.
    fn merge_subscribers(&mut self);

    /// Called by an Effect or Memo indirectly by reading the current value.
    fn subscribe(&mut self, entity: Entity);
}

/// A LazySignalsState is known as a cell in a propagator network. It may also be referred to as
/// state. Using the label LazySignalsState because Cell often means another thing.
/// Mutable is used by futures-signals for the same data-wrapping purpose, but in our case, the
/// cells are mutated by sending signal explicitly (i.e. calling merge_next and adding SendSignal).
///
/// Some convenience types provided:
/// LazyImmutableBool, LazyImmutableInt, LazyImmutableFloat, LazyImmutableStr, LazyImmutableUnit.
///
/// The subscriber set is built from the sources/triggers of computed memos and effects, so it does
/// not have to be serialized, which is good because the SparseSet doesn't seem to do Reflect.
///
/// This LazySignalsState component is lazy. Other forms are left as an exercise for the reader.
#[derive(Component, Reflect)]
#[reflect(Component, LazySignalsObservable)]
pub struct LazySignalsState<T: LazySignalsData> {
    result: LazySignalsResult<T>,
    next_value: LazySignalsResult<T>,
    triggered: bool,
    #[reflect(ignore)]
    subscribers: EntitySet,
    #[reflect(ignore)]
    next_subscribers: EntitySet,
}

impl<T: LazySignalsData> LazySignalsState<T> {
    pub fn new(result: LazySignalsResult<T>) -> Self {
        Self {
            result,
            next_value: LazySignalsResult {
                data: None,
                error: Some(LazySignalsError::NoNextValue),
            },
            triggered: false,
            subscribers: empty_set(),
            next_subscribers: empty_set(),
        }
    }
}

impl<T: LazySignalsData> LazySignalsImmutable for LazySignalsState<T> {
    type DataType = T;

    fn error(&self) -> Option<LazySignalsError> {
        clone_data(&self.result).error
    }

    fn get(&self) -> Option<Self::DataType> {
        clone_data(&self.result).data
    }

    fn merge_next(&mut self, next_value: LazySignalsResult<T>, triggered: bool) {
        self.next_value = next_value;
        self.triggered = triggered;
    }

    fn update(&mut self, next: LazySignalsResult<Self::DataType>) -> bool {
        let changed = self.result != next;
        self.result = next;
        changed
    }
}

impl<T: LazySignalsData> LazySignalsObservable for LazySignalsState<T> {
    fn append_none(&mut self, args: &mut DynamicTuple) {
        args.insert::<Option<T>>(None);
    }

    fn copy_data(&mut self, caller: Entity, args: &mut DynamicTuple) {
        insert_data(args, &self.result);

        self.subscribe(caller);
    }

    fn get_subscribers(&self) -> Vec<Entity> {
        let mut subs = Vec::<Entity>::new();

        // copy the subscribers into the output vector
        subs.extend(self.subscribers.indices());
        trace!("-found subs {:?}", self.subscribers);
        subs
    }

    fn merge(&mut self) -> MaybeFlaggedEntities {
        let mut changed = false;
        let triggered = self.triggered;

        // output vector for downstream subscribers to process next
        let mut subs = Vec::<Entity>::new();

        // whether or not to overwrite the existing info
        let doo_eet = match &self.next_value.error {
            Some(err) =>
                // always merge errors
                match err {
                    LazySignalsError::NoSignalError => true,

                    // not a real error, nothing to merge...
                    LazySignalsError::NoNextValue => false,

                    LazySignalsError::ReadError(_) => true,
                }
            None =>
                // if there is no error, then compare the data values
                match &self.next_value.data {
                    Some(next) => {
                        trace!("next exists");
                        match &self.result.data {
                            Some(data) => {
                                trace!("data exists");
                                if data != next {
                                    // the new data is different
                                    trace!("data != next");
                                    changed = true;
                                    true
                                } else {
                                    // the new data is the same as current
                                    false
                                }
                            }
                            None => {
                                // currently no data but there's data to merge which means changed
                                changed = true;
                                true
                            }
                        }
                    }
                    // there is no next value, so clear out the current value
                    None => self.result.data.is_some(),
                }
        };

        // overwrite the value
        if doo_eet {
            self.result = clone_data(&self.next_value);
            self.next_value = LazySignalsResult {
                data: None,
                error: Some(LazySignalsError::NoNextValue),
            };
        }

        // return a list of subscribers
        if doo_eet || triggered {
            // copy the subscribers into the output vector
            subs = self.get_subscribers();

            // clear the local subscriber set which will be replenished by each subscriber if
            // it calls the value method later
            self.subscribers.clear();

            // trigger is processed, so reset the flag
            self.triggered = false;
        }
        Some((subs, changed, triggered))
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
}
