use std::{ any::TypeId, sync::RwLockReadGuard };

use bevy::{
    ecs::entity::Entity,
    prelude::*,
    ptr::PtrMut,
    reflect::{ ReflectFromPtr, TypeRegistry },
};

use crate::ReflectUntypedObservable;

pub(crate) fn make_reflect_from_ptr(
    type_id: TypeId,
    type_registry: &RwLockReadGuard<TypeRegistry>
) -> ReflectFromPtr {
    // the reflect_data is used to build a strategy to dereference a pointer to the component
    let reflect_data = type_registry.get(type_id).unwrap();

    // we're going to get a pointer to the component, so we'll need this
    reflect_data.data::<ReflectFromPtr>().unwrap().clone()
}

// initialize the subscriber sets for all new Signals and Memos
pub(crate) fn enter_malkovich_world(
    subscriber: Entity,
    ptr_mut: PtrMut,
    reflect_from_ptr: &ReflectFromPtr,
    type_registry: &RwLockReadGuard<TypeRegistry>
) {
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
    untyped_observable.subscribe(subscriber);
    info!("-subscribed {:?}", subscriber);
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
    info!("-merged subscribers");
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
