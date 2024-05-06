use std::{ any::TypeId, sync::RwLockReadGuard };

use bevy::{
    ecs::{ component::ComponentId, entity::Entity, world::EntityWorldMut },
    prelude::*,
    ptr::PtrMut,
    reflect::{ DynamicTuple, ReflectFromPtr, TypeRegistry },
};

use crate::signals::*;

pub(crate) fn make_reflect_from_ptr(
    type_id: TypeId,
    type_registry: &RwLockReadGuard<TypeRegistry>
) -> ReflectFromPtr {
    // the reflect_data is used to build a strategy to dereference a pointer to the component
    let reflect_data = type_registry.get(type_id).unwrap();

    // we're going to get a pointer to the component, so we'll need this
    reflect_data.data::<ReflectFromPtr>().unwrap().clone()
}

// add a subscriber
pub(crate) fn enter_malkovich_world(
    source: &mut EntityWorldMut,
    subscriber: &Entity,
    component_id: &ComponentId,
    type_id: &TypeId,
    type_registry: &RwLockReadGuard<TypeRegistry>
) {
    let component_id = *component_id;
    // get the source Immutable component as an ECS change detection handle
    let mut mut_untyped = source.get_mut_by_id(component_id).unwrap();

    // ...and convert that into a pointer
    let ptr_mut = mut_untyped.as_mut();

    // insert arcane wizardry here
    let reflect_from_ptr = make_reflect_from_ptr(*type_id, type_registry);

    // the following boilerplate required due to rules about passing trait objects around

    // safety: `value` implements reflected trait `UntypedObservable`, what for `ReflectFromPtr`
    let value = unsafe { reflect_from_ptr.as_reflect_mut(ptr_mut) };

    // meet the new flesh
    let reflect_untyped_observable = type_registry
        .get_type_data::<ReflectUntypedObservable>(value.type_id())
        .unwrap();

    // same as the old flesh
    let untyped_observable = reflect_untyped_observable.get_mut(value).unwrap();

    // make it so!
    info!("-subscribing {:?}", subscriber);
    untyped_observable.subscribe(*subscriber);
}

// get the list of subscribers
pub(crate) fn this_is_bat_country(
    source: &mut EntityWorldMut,
    component_id: &ComponentId,
    type_id: &TypeId,
    type_registry: &RwLockReadGuard<TypeRegistry>
) -> Vec<Entity> {
    let component_id = *component_id;
    // get the source Immutable component as an ECS change detection handle
    let mut mut_untyped = source.get_mut_by_id(component_id).unwrap();

    // ...and convert that into a pointer
    let ptr_mut = mut_untyped.as_mut();

    // insert arcane wizardry here
    let reflect_from_ptr = make_reflect_from_ptr(*type_id, type_registry);

    // the following boilerplate required due to rules about passing trait objects around

    // safety: `value` implements reflected trait `UntypedObservable`, what for `ReflectFromPtr`
    let value = unsafe { reflect_from_ptr.as_reflect_mut(ptr_mut) };

    // the sun grew dark and cold
    let reflect_untyped_observable = type_registry
        .get_type_data::<ReflectUntypedObservable>(value.type_id())
        .unwrap();

    // the seas boiled
    let untyped_observable = reflect_untyped_observable.get_mut(value).unwrap();

    // do the dang thing
    untyped_observable.get_subscribers()
}

// merge subscribers
pub(crate) fn long_live_the_new_flesh(
    ptr_mut: PtrMut,
    reflect_from_ptr: &ReflectFromPtr,
    type_registry: &RwLockReadGuard<TypeRegistry>
) {
    // the following boilerplate required due to rules about passing trait objects around

    // safety: `value` implements reflected trait `UntypedObservable`, what for `ReflectFromPtr`
    let value = unsafe { reflect_from_ptr.as_reflect_mut(ptr_mut) };

    // dogs and cats
    let reflect_untyped_observable = type_registry
        .get_type_data::<ReflectUntypedObservable>(value.type_id())
        .unwrap();

    // living together
    let untyped_observable = reflect_untyped_observable.get_mut(value).unwrap();

    // engage!
    untyped_observable.merge_subscribers();
}

// mut (apply the next value to) the Immutable
pub(crate) fn the_abyss_gazes_into_you(
    ptr_mut: PtrMut,
    reflect_from_ptr: &ReflectFromPtr,
    type_registry: &RwLockReadGuard<TypeRegistry>
) -> Vec<Entity> {
    // the following boilerplate required due to rules about passing trait objects around

    // safety: `value` implements reflected trait `UntypedObservable`, what for `ReflectFromPtr`
    let value = unsafe { reflect_from_ptr.as_reflect_mut(ptr_mut) };

    // the sun grew dark and cold
    let reflect_untyped_observable = type_registry
        .get_type_data::<ReflectUntypedObservable>(value.type_id())
        .unwrap();

    // the seas boiled
    let untyped_observable = reflect_untyped_observable.get_mut(value).unwrap();

    // do the dang thing
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
    let component_id = *component_id;
    // get the source Immutable component as an ECS change detection handle
    let mut mut_untyped = source.get_mut_by_id(component_id).unwrap();

    // ...and convert that into a pointer
    let ptr_mut = mut_untyped.as_mut();

    // insert arcane wizardry here
    let reflect_from_ptr = make_reflect_from_ptr(*type_id, type_registry);

    // the following boilerplate required due to rules about passing trait objects around

    // safety: `value` implements reflected trait `UntypedObservable`, what for `ReflectFromPtr`
    let value = unsafe { reflect_from_ptr.as_reflect_mut(ptr_mut) };

    // in their house at R'lyeh
    let reflect_untyped_observable = type_registry
        .get_type_data::<ReflectUntypedObservable>(value.type_id())
        .unwrap();

    // dead Cthulhu waits
    let untyped_observable = reflect_untyped_observable.get_mut(value).unwrap();

    // dreaming
    untyped_observable.copy_data(*target, params);
}
