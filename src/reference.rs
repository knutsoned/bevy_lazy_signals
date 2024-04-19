use std::marker::PhantomData;

use bevy_ecs::prelude::*;

use crate::types::*;

/// Based on bevy_rx:
/// Lazy signal propagation
impl<T: Send + Sync + 'static> Mutable<T> {
    #[allow(clippy::new_ret_no_self)]
    pub fn new<S>(rctx: &mut ReactiveContext<S>, data: T) -> Entity {
        rctx.reactive_state
            .spawn(Self {
                data,
                subscribers: Vec::new(),
            })
            .id()
    }
    pub fn subscribe(&mut self, entity: Entity) {
        self.subscribers.push(entity);
    }

    pub fn data(&self) -> &T {
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
        // TODO this part
    }
    pub fn send_signal(world: &mut World, signal_target: Entity, value: T) {
        // TODO review
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
            reactive_entity: entity,
            p: PhantomData,
        }
    }

    pub fn read<'r, S>(&self, rctx: &'r mut ReactiveContext<S>) -> &'r T {
        rctx.reactive_state.get::<Mutable<T>>(self.reactive_entity).unwrap().data()
    }
}

impl<S> Default for ReactiveContext<S> {
    fn default() -> Self {
        let mut world = World::default();
        world.init_resource::<DeferredEffects>();
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

    pub fn new_effect<M>(
        &mut self,
        observable: impl Observable,
        effect_system: impl IntoSystem<(), (), M>
    ) -> Effect {
        Effect::new_deferred(self, observable, effect_system)
    }

    pub fn effect_system(&self, effect: Effect) -> Option<&dyn System<In = (), Out = ()>> {
        self.reactive_state
            .get::<DeferredEffect>(effect.reactive_entity)
            .and_then(|effect| effect.system())
    }
}

impl<T: Send + Sync + FnMut(&mut World, &mut Vec<Entity>)> DeriveFn for T {}

impl RxMemo {
    pub fn new<C: Clone + Send + Sync + PartialEq + 'static, D: MemoQuery<C> + 'static>(
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

    pub fn execute(&mut self, world: &mut World, stack: &mut Vec<Entity>) {
        (self.function)(world, stack);
    }
}

impl<T: Send + Sync + PartialEq> Observable for Signal<T> {
    type DataType = T;
    fn reactive_entity(&self) -> Entity {
        self.reactive_entity
    }
}

impl<T: Send + Sync + PartialEq> Clone for Signal<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Send + Sync + PartialEq> Copy for Signal<T> {}

impl<T: Clone + Send + Sync + PartialEq> Signal<T> {
    pub fn new<S>(rctx: &mut ReactiveContext<S>, initial_value: T) -> Self {
        Self {
            reactive_entity: Mutable::new(rctx, initial_value),
            p: PhantomData,
        }
    }

    pub fn read<'r, S>(&self, rctx: &'r mut ReactiveContext<S>) -> &'r T {
        rctx.reactive_state.get::<Mutable<T>>(self.reactive_entity).unwrap().data()
    }

    /// See [`ReactiveContext::send_signal`].
    #[inline]
    pub fn send<S>(&self, rctx: &mut ReactiveContext<S>, value: T) {
        Mutable::send_signal(&mut rctx.reactive_state, self.reactive_entity, value)
    }
}

impl DeferredEffect {
    pub fn new<M>(system: impl IntoSystem<(), (), M>) -> Self {
        Self {
            system: EffectSystem::new(system),
        }
    }

    pub fn run(&mut self, main_world: &mut World) {
        self.system.run(main_world);
    }

    pub fn system(&self) -> Option<&dyn System<In = (), Out = ()>> {
        match &self.system {
            EffectSystem::Empty => None,
            EffectSystem::New(s) | EffectSystem::Initialized(s) => Some(s.as_ref()),
        }
    }
}

impl Effect {
    pub fn new_deferred<M, S>(
        rctx: &mut ReactiveContext<S>,
        observable: impl Observable,
        effect_system: impl IntoSystem<(), (), M>
    ) -> Self {
        let reactive_entity = observable.reactive_entity();
        rctx.reactive_state.entity_mut(reactive_entity).insert(DeferredEffect::new(effect_system));

        Self { reactive_entity }
    }

    pub fn get<'r, S>(
        &self,
        rctx: &'r mut ReactiveContext<S>
    ) -> Option<&'r dyn System<In = (), Out = ()>> {
        rctx.reactive_state.get::<DeferredEffect>(self.reactive_entity).unwrap().system()
    }
}

impl EffectSystem {
    pub fn new<M>(system: impl IntoSystem<(), (), M>) -> Self {
        Self::New(Box::new(IntoSystem::into_system(system)))
    }

    pub fn run(&mut self, world: &mut World) {
        let mut system = match std::mem::take(self) {
            EffectSystem::Empty => {
                return;
            }
            EffectSystem::New(mut system) => {
                system.initialize(world);
                system
            }
            EffectSystem::Initialized(system) => system,
        };
        system.run((), world);
        system.apply_deferred(world);
        *self = EffectSystem::Initialized(system);
    }
}
