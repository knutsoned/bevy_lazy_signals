use std::{ any::TypeId, sync::RwLockReadGuard };

use bevy::{
    ecs::{
        change_detection::MutUntyped,
        component::ComponentId,
        entity::Entity,
        world::EntityWorldMut,
    },
    prelude::*,
    reflect::{ DynamicTuple, ReflectFromPtr, TypeRegistry },
};

use crate::framework::*;

// given a mutable reference to a LazyImmutable component instance, make an UntypedObservable
pub fn make_observable<'a>(
    mut_untyped: &'a mut MutUntyped,
    type_id: &TypeId,
    type_registry: &RwLockReadGuard<TypeRegistry>
) -> &'a mut dyn LazySignalsObservable {
    // convert into a pointer
    let ptr_mut = mut_untyped.as_mut();

    // the reflect_data is used to build a strategy to dereference a pointer to the component
    let reflect_data = type_registry.get(*type_id).unwrap();

    // we're going to get a pointer to the component, so we'll need this
    let reflect_from_ptr = reflect_data.data::<ReflectFromPtr>().unwrap().clone();

    // safety: `value` implements reflected trait `UntypedObservable`, what for `ReflectFromPtr`
    let value = unsafe { reflect_from_ptr.as_reflect_mut(ptr_mut) };

    // the sun grew dark and cold
    let reflect_untyped_observable = type_registry
        .get_type_data::<ReflectLazySignalsObservable>(value.type_id())
        .unwrap();

    // the seas boiled
    reflect_untyped_observable.get_mut(value).unwrap()
}

// add a subscriber
pub(crate) fn enter_malkovich_world(
    source: &mut EntityWorldMut,
    subscriber: &Entity,
    component_id: &ComponentId,
    type_id: &TypeId,
    type_registry: &RwLockReadGuard<TypeRegistry>
) {
    let entity = source.id();

    // the following boilerplate required due to rules about returning local variables

    // get the source Immutable component as an ECS change detection handle
    let mut mut_untyped = source.get_mut_by_id(*component_id).unwrap();

    // ...and convert that into a trait object
    let observable = make_observable(&mut mut_untyped, type_id, type_registry);

    // make it so!
    info!("-subscribing {:?} to {:?}", subscriber, entity);
    observable.subscribe(*subscriber);
}

// get a copy of the list of subscribers
pub(crate) fn this_is_bat_country(
    source: &mut EntityWorldMut,
    component_id: &ComponentId,
    type_id: &TypeId,
    type_registry: &RwLockReadGuard<TypeRegistry>
) -> Vec<Entity> {
    // the following boilerplate required due to rules about returning local variables

    // get the source Immutable component as an ECS change detection handle
    let mut mut_untyped = source.get_mut_by_id(*component_id).unwrap();

    // ...and convert that into a trait object
    let observable = make_observable(&mut mut_untyped, type_id, type_registry);

    // I want to go fast!
    observable.get_subscribers()
}

// merge subscribers
pub(crate) fn long_live_the_new_flesh(
    source: &mut EntityWorldMut,
    component_id: &ComponentId,
    type_id: &TypeId,
    type_registry: &RwLockReadGuard<TypeRegistry>
) {
    // the following boilerplate required due to rules about returning local variables

    // get the source Immutable component as an ECS change detection handle
    let mut mut_untyped = source.get_mut_by_id(*component_id).unwrap();

    // ...and convert that into a trait object
    let observable = make_observable(&mut mut_untyped, type_id, type_registry);

    // engage!
    observable.merge_subscribers();
}

// mut (apply the next value to) the Immutable
pub(crate) fn the_abyss_gazes_into_you(
    source: &mut EntityWorldMut,
    component_id: &ComponentId,
    type_id: &TypeId,
    type_registry: &RwLockReadGuard<TypeRegistry>
) -> (Vec<Entity>, bool) {
    // the following boilerplate required due to rules about returning local variables

    // get the source Immutable component as an ECS change detection handle
    let mut mut_untyped = source.get_mut_by_id(*component_id).unwrap();

    // ...and convert that into a trait object
    let observable = make_observable(&mut mut_untyped, type_id, type_registry);

    // give me warp in the factor of uh 5, 6, 7, 8
    let triggered = observable.is_triggered();
    let subs = observable.merge();

    // TODO make an enum for the return type
    // TODO also figure out how to pass errors up from functions
    //      without having to handle the error in each function
    (subs, triggered)
}

// copy untyped data into a dynamic tuple
pub(crate) fn ph_nglui_mglw_nafh_cthulhu_r_lyeh_wgah_nagl_fhtagn(
    source: &mut EntityWorldMut,
    target: &Entity,
    params: &mut DynamicTuple,
    component_id: &ComponentId,
    type_id: &TypeId,
    type_registry: &RwLockReadGuard<TypeRegistry>
) {
    // the following boilerplate required due to rules about returning local variables

    // get the source Immutable component as an ECS change detection handle
    let mut mut_untyped = source.get_mut_by_id(*component_id).unwrap();

    // ...and convert that into a trait object
    let observable = make_observable(&mut mut_untyped, type_id, type_registry);

    // please clap
    observable.copy_data(*target, params);
}
