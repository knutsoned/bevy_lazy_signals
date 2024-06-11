use bevy::prelude::*;
use crate::{ framework::*, lazy_immutable::LazySignalsState };

/// ## Bundles
#[derive(Bundle)]
pub struct ComputedBundle<R: LazySignalsData> {
    state: LazySignalsState<R>,
    meta: ImmutableState,
    context: ComputedImmutable,
    init: InitDependencies,
}

impl<R: LazySignalsData> ComputedBundle<R> {
    pub fn from_function<P: LazySignalsArgs>(
        function: Mutex<Box<dyn ComputedContext>>,
        sources: Vec<Entity>,
        component_id: ComponentId
    ) -> ComputedBundle<R> {
        ComputedBundle::<R> {
            state: LazySignalsState::<R>::new(None),
            meta: ImmutableState { component_id },
            context: ComputedImmutable {
                function,
                sources,
                args_type: TypeId::of::<P>(),
                result_type: TypeId::of::<LazySignalsState<R>>(),
            },
            init: InitDependencies,
        }
    }
}

#[derive(Bundle)]
pub struct EffectBundle {
    context: LazyEffect,
    init: InitDependencies,
}

impl EffectBundle {
    pub fn from_function<P: LazySignalsArgs>(
        function: EffectContext,
        sources: Vec<Entity>,
        triggers: Vec<Entity>
    ) -> EffectBundle {
        EffectBundle {
            context: LazyEffect {
                function,
                sources,
                triggers,
                args_type: TypeId::of::<P>(),
            },
            init: InitDependencies,
        }
    }
}

#[derive(Bundle)]
pub struct StateBundle<T: LazySignalsData> {
    state: LazySignalsState<T>,
    meta: ImmutableState,
}

impl<T: LazySignalsData> StateBundle<T> {
    pub fn from_value(data: T, component_id: ComponentId) -> StateBundle<T> {
        StateBundle {
            state: LazySignalsState::<T>::new(Some(Ok(data))),
            meta: ImmutableState { component_id },
        }
    }
}
