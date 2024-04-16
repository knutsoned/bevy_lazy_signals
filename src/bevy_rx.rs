use std::{ marker::PhantomData, ops::{ Deref, DerefMut } };

use bevy_ecs::{ prelude::*, system::SystemParam };
use bevy_utils::all_tuples_with_size;

/// Derived from bevy_rx:
/// Generalizes over multiple bevy reactive components the user has access to, that are ultimately
/// just handles containing the entity in the [`ReactiveContext`].
pub trait Observable: Copy + Send + Sync + 'static {
    type DataType: PartialEq + Send + Sync + 'static;
    fn reactive_entity(&self) -> Entity;
}

/// Derived from bevy_rx RxObservableData:
/// The core reactive primitive that holds data, and a list of subscribers that are invoked when the
/// data changes.
#[derive(Component)]
pub(crate) struct Mutable<T> {
    pub data: T,
    pub subscribers: Vec<Entity>,
}

impl<T: Send + Sync + 'static> Mutable<T> {
    #[allow(clippy::new_ret_no_self)]
    pub(crate) fn new<S>(rctx: &mut ReactiveContext<S>, data: T) -> Entity {
        rctx.reactive_state
            .spawn(Self {
                data,
                subscribers: Vec::new(),
            })
            .id()
    }
    pub(crate) fn subscribe(&mut self, entity: Entity) {
        self.subscribers.push(entity);
    }

    pub(crate) fn data(&self) -> &T {
        &self.data
    }
}

impl<T: Clone + PartialEq + Send + Sync + 'static> Mutable<T> {
    /// Update the reactive value, and push subscribers onto the stack.
    pub fn update_value(
        rx_world: &mut World,
        stack: &mut Vec<Entity>,
        observable: Entity,
        value: T
    ) {
        if let Some(mut reactive) = rx_world.get_mut::<Mutable<T>>(observable) {
            if reactive.data == value {
                return; // Diff the value and early exit if no change.
            }
            reactive.data = value.clone();
            // Remove all subscribers from this entity. If any of these subscribers end up
            // using this data, they will resubscribe themselves. This is the
            // auto-unsubscribe part of the reactive implementation.
            //
            // We push these subscribers on the stack, so that they can be executed, just
            // like this one was. We use a stack instead of recursion to avoid stack
            // overflow.
            stack.append(&mut reactive.subscribers);
        } else {
            rx_world.entity_mut(observable).insert(Mutable {
                data: value.clone(),
                subscribers: Default::default(),
            });
        }
        /* TODO: effects
        if rx_world.get_mut::<RxDeferredEffect>(observable).is_some() {
            rx_world.resource_mut::<RxDeferredEffects>().push::<T>(observable);
        }
        */
    }
    /// Update value of this reactive entity, additionally, trigger all subscribers. The
    /// [`Reactive`] component will be added if it is missing.
    pub(crate) fn send_signal(world: &mut World, signal_target: Entity, value: T) {
        let mut stack = Vec::new();

        Self::update_value(world, &mut stack, signal_target, value);

        while let Some(sub) = stack.pop() {
            if let Some(mut calculation) = world.entity_mut(sub).take::<RxMemo>() {
                calculation.execute(world, &mut stack);
                world.entity_mut(sub).insert(calculation);
            }
        }
    }
}

/// Derived from bevy_rx:
/// A reactive calculation that is run on observable data, and memoized (cached).
///
/// This component lives in the reactive world and holds the user calculation function. [`Memo`] is
/// the user-facing counterpart in the main world, which is a lightweight handle to access this
/// mirror component.
///
/// This component is expected to be on an entity with an [`crate::Mutable`] component. The
/// contained function can be called without the caller knowing any type information, and will
/// update the associated [`Mutable`] component.
/// A reactive value that is automatically recalculated and memoized (cached).
///
/// The value can only be read through the [`ReactiveContext`].
#[derive(Debug, Component)]
pub struct Memo<T: Send + Sync + 'static> {
    pub(crate) reactor_entity: Entity,
    pub(crate) p: PhantomData<T>,
}

impl<T: Send + Sync + PartialEq> Observable for Memo<T> {
    type DataType = T;
    fn reactive_entity(&self) -> Entity {
        self.reactor_entity
    }
}

impl<T: Send + Sync> Clone for Memo<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Send + Sync> Copy for Memo<T> {}

impl<T: Clone + PartialEq + Send + Sync> Memo<T> {
    pub fn new<S, D: MemoQuery<T>>(
        rctx: &mut ReactiveContext<S>,
        input_deps: D,
        derive_fn: impl (Fn(D::Query<'_>) -> T) + Send + Sync + Clone + 'static
    ) -> Self {
        let entity = rctx.reactive_state.spawn_empty().id();
        let mut derived = RxMemo::new(entity, input_deps, derive_fn);
        derived.execute(&mut rctx.reactive_state, &mut Vec::new());
        rctx.reactive_state.entity_mut(entity).insert(derived);
        Self {
            reactor_entity: entity,
            p: PhantomData,
        }
    }

    pub fn read<'r, S>(&self, rctx: &'r mut ReactiveContext<S>) -> &'r T {
        rctx.reactive_state.get::<Mutable<T>>(self.reactor_entity).unwrap().data()
    }
}

#[derive(Component)]
pub(crate) struct RxMemo {
    function: Box<dyn DeriveFn>,
}

trait DeriveFn: Send + Sync + FnMut(&mut World, &mut Vec<Entity>) {}
impl<T: Send + Sync + FnMut(&mut World, &mut Vec<Entity>)> DeriveFn for T {}

impl RxMemo {
    pub(crate) fn new<C: Clone + Send + Sync + PartialEq + 'static, D: MemoQuery<C> + 'static>(
        entity: Entity,
        input_deps: D,
        derive_fn: impl (Fn(D::Query<'_>) -> C) + Clone + Send + Sync + 'static
    ) -> Self {
        let function = move |world: &mut World, stack: &mut Vec<Entity>| {
            let computed_value = D::read_and_derive(world, entity, derive_fn.clone(), input_deps);
            if let Some(computed_value) = computed_value {
                Mutable::update_value(world, stack, entity, computed_value);
            }
        };
        let function = Box::new(function);
        Self { function }
    }

    pub(crate) fn execute(&mut self, world: &mut World, stack: &mut Vec<Entity>) {
        (self.function)(world, stack);
    }
}

/// Implemented on tuples to be used for querying
pub trait MemoQuery<T>: Copy + Send + Sync + 'static {
    type Query<'a>;
    fn read_and_derive(
        world: &mut World,
        reader: Entity,
        derive_fn: impl Fn(Self::Query<'_>) -> T,
        input_deps: Self
    ) -> Option<T>;
}

macro_rules! impl_CalcQuery {
    ($N:expr, $(($T:ident, $I:ident)),*) => {
        impl<$($T: Observable), *, D> MemoQuery<D> for ($($T,)*) {
            type Query<'a> = ($(&'a $T::DataType,)*);

            fn read_and_derive(
                world: &mut World,
                reader: Entity,
                derive_fn: impl Fn(Self::Query<'_>) -> D,
                entities: Self,
            ) -> Option<D> {
                let ($($I,)*) = entities;
                let entities = [$($I.reactive_entity(),)*];

                // Note this is left to unwrap intentionally. If aliased mutability happens, this is
                // an error and should panic. If we were to early exit here, it would lead to
                // harder-to-debug errors down the line.
                let [$(mut $I,)*] = world.get_many_entities_mut(entities).unwrap();

                $($I.get_mut::<Mutable<$T::DataType>>()?.subscribe(reader);)*

                Some(derive_fn((
                    $($I.get::<Mutable<$T::DataType>>()?.data(),)*
                )))
            }
        }
    };
}

all_tuples_with_size!(impl_CalcQuery, 1, 32, T, s);

/// Derived from bevy_rx:
/// A reactive component that can updated with new values or read through the [`ReactiveContext`].
#[derive(Debug, Component)]
pub struct Signal<T: Send + Sync + 'static> {
    reactor_entity: Entity,
    p: PhantomData<T>,
}

impl<T: Send + Sync + PartialEq> Observable for Signal<T> {
    type DataType = T;
    fn reactive_entity(&self) -> Entity {
        self.reactor_entity
    }
}

impl<T: Send + Sync + PartialEq> Clone for Signal<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Send + Sync + PartialEq> Copy for Signal<T> {}

impl<T: Clone + Send + Sync + PartialEq> Signal<T> {
    pub(crate) fn new<S>(rctx: &mut ReactiveContext<S>, initial_value: T) -> Self {
        Self {
            reactor_entity: Mutable::new(rctx, initial_value),
            p: PhantomData,
        }
    }

    pub fn read<'r, S>(&self, rctx: &'r mut ReactiveContext<S>) -> &'r T {
        rctx.reactive_state.get::<Mutable<T>>(self.reactor_entity).unwrap().data()
    }

    /// See [`ReactiveContext::send_signal`].
    #[inline]
    pub fn send<S>(&self, rctx: &mut ReactiveContext<S>, value: T) {
        Mutable::send_signal(&mut rctx.reactive_state, self.reactor_entity, value)
    }
}

/// Derived from bevy_rx:
/// A system param to make accessing the [`ReactiveContext`] less verbose.
#[derive(SystemParam)]
pub struct Reactor<'w>(ResMut<'w, ReactiveContext<World>>);
impl<'w> Deref for Reactor<'w> {
    type Target = ReactiveContext<World>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<'w> DerefMut for Reactor<'w> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Derived from bevy_rx:
/// Contains all reactive state. A bevy world is used because it makes it easy to store statically
/// typed data in a type erased container.
#[derive(Resource)]
pub struct ReactiveContext<S> {
    reactive_state: World,
    outside_state: PhantomData<S>,
}

#[allow(unused_mut)]
impl<S> Default for ReactiveContext<S> {
    fn default() -> Self {
        let mut world = World::default();
        // TODO: effects
        //world.init_resource::<RxDeferredEffects>();
        Self {
            reactive_state: world,
            outside_state: PhantomData,
        }
    }
}

impl<S> ReactiveContext<S> {
    /// Returns a reference to the current value of the provided observable. The observable is any
    /// reactive handle that has a value, like a [`Signal`] or a [`Derived`].
    pub fn read<T: Send + Sync + PartialEq + 'static, O: Observable<DataType = T>>(
        &mut self,
        observable: O
    ) -> &T {
        // get the obs data from the world
        // add the reader to the obs data's subs
        self.reactive_state.get::<Mutable<T>>(observable.reactive_entity()).unwrap().data()
    }

    /// Send a signal, and run the reaction graph to completion.
    ///
    /// Potentially expensive operation that will write a value to this [`Signal`]`. This will cause
    /// all reactive subscribers of this observable to recompute their own values, which can cause
    /// all of its subscribers to recompute, etc.
    pub fn send_signal<T: Clone + Send + Sync + PartialEq + 'static>(
        &mut self,
        signal: Signal<T>,
        value: T
    ) {
        Mutable::send_signal(&mut self.reactive_state, signal.reactive_entity(), value)
    }

    pub fn new_signal<T: Clone + Send + Sync + PartialEq + 'static>(
        &mut self,
        initial_value: T
    ) -> Signal<T> {
        Signal::new(self, initial_value)
    }

    pub fn new_memo<T: Clone + Send + Sync + PartialEq + 'static, C: MemoQuery<T> + 'static>(
        &mut self,
        calculation_query: C,
        derive_fn: impl (Fn(C::Query<'_>) -> T) + Send + Sync + Clone + 'static
    ) -> Memo<T> {
        Memo::new(self, calculation_query, derive_fn)
    }

    /* TODO: effects
    pub fn new_deferred_effect<M>(
        &mut self,
        observable: impl Observable,
        effect_system: impl IntoSystem<(), (), M>
    ) -> Effect {
        Effect::new_deferred(self, observable, effect_system)
    }

    pub fn effect_system(&self, effect: Effect) -> Option<&dyn System<In = (), Out = ()>> {
        self.reactive_state
            .get::<RxDeferredEffect>(effect.reactor_entity)
            .and_then(|effect| effect.system())
    }
    */
}
