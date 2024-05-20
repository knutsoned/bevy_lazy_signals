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

use crate::signals::*;

// given a mutable reference to a LazyImmutable component instance, make a UntypedObservable
pub fn make_untyped_observable<'a>(
    mut_untyped: &'a mut MutUntyped,
    type_id: &TypeId,
    type_registry: &RwLockReadGuard<TypeRegistry>
) -> &'a mut dyn UntypedObservable {
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
        .get_type_data::<ReflectUntypedObservable>(value.type_id())
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
    // FIXME make sure this is necessary, otherwise
    // change make_untyped_mut to accept the entity and component ids instead of mut_untyped

    // get the source Immutable component as an ECS change detection handle
    let mut mut_untyped = source.get_mut_by_id(*component_id).unwrap();

    // ...and convert that into a trait object
    let untyped_observable = make_untyped_observable(&mut mut_untyped, type_id, type_registry);

    // make it so!
    info!("-subscribing {:?} to {:?}", subscriber, entity);
    untyped_observable.subscribe(*subscriber);
}

// get the list of subscribers
pub(crate) fn this_is_bat_country(
    source: &mut EntityWorldMut,
    component_id: &ComponentId,
    type_id: &TypeId,
    type_registry: &RwLockReadGuard<TypeRegistry>
) -> Vec<Entity> {
    // the following boilerplate required due to rules about returning local variables
    // FIXME make sure this is necessary, otherwise
    // change make_untyped_mut to accept the entity and component ids instead of mut_untyped

    // get the source Immutable component as an ECS change detection handle
    let mut mut_untyped = source.get_mut_by_id(*component_id).unwrap();

    // ...and convert that into a trait object
    let untyped_observable = make_untyped_observable(&mut mut_untyped, type_id, type_registry);

    // I want to go fast!
    untyped_observable.get_subscribers()
}

// merge subscribers
pub(crate) fn long_live_the_new_flesh(
    source: &mut EntityWorldMut,
    component_id: &ComponentId,
    type_id: &TypeId,
    type_registry: &RwLockReadGuard<TypeRegistry>
) {
    // the following boilerplate required due to rules about returning local variables
    // FIXME make sure this is necessary, otherwise
    // change make_untyped_mut to accept the entity and component ids instead of mut_untyped

    // get the source Immutable component as an ECS change detection handle
    let mut mut_untyped = source.get_mut_by_id(*component_id).unwrap();

    // ...and convert that into a trait object
    let untyped_observable = make_untyped_observable(&mut mut_untyped, type_id, type_registry);

    // engage!
    untyped_observable.merge_subscribers();
}

// mut (apply the next value to) the Immutable
pub(crate) fn the_abyss_gazes_into_you(
    source: &mut EntityWorldMut,
    component_id: &ComponentId,
    type_id: &TypeId,
    type_registry: &RwLockReadGuard<TypeRegistry>
) -> Vec<Entity> {
    // the following boilerplate required due to rules about returning local variables
    // FIXME make sure this is necessary, otherwise
    // change make_untyped_mut to accept the entity and component ids instead of mut_untyped

    // get the source Immutable component as an ECS change detection handle
    let mut mut_untyped = source.get_mut_by_id(*component_id).unwrap();

    // ...and convert that into a trait object
    let untyped_observable = make_untyped_observable(&mut mut_untyped, type_id, type_registry);

    // give me warp in the factor of uh 5, 6, 7, 8
    untyped_observable.merge()
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
    // FIXME make sure this is necessary, otherwise
    // change make_untyped_mut to accept the entity and component ids instead of mut_untyped

    // get the source Immutable component as an ECS change detection handle
    let mut mut_untyped = source.get_mut_by_id(*component_id).unwrap();

    // ...and convert that into a trait object
    let untyped_observable = make_untyped_observable(&mut mut_untyped, type_id, type_registry);

    // please clap
    untyped_observable.copy_data(*target, params);
}
