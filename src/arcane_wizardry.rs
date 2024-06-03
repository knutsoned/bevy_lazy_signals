use std::{ any::TypeId, sync::RwLockReadGuard };

use bevy::{
    ecs::{
        change_detection::MutUntyped,
        component::ComponentId,
        entity::Entity,
        world::EntityWorldMut,
    },
    reflect::{ DynamicTuple, ReflectFromPtr, TypeRegistry },
};

use crate::framework::*;

// given a mutable reference to a LazyImmutable component instance, make an UntypedObservable
pub fn ph_nglui_mglw_nafh_cthulhu_r_lyeh_wgah_nagl_fhtagn<'a>(
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
    let reflect_observable = type_registry
        .get_type_data::<ReflectLazySignalsObservable>(value.type_id())
        .unwrap();

    // the seas boiled
    reflect_observable.get_mut(value).unwrap()
}

pub fn run_observable_method(
    entity: &mut EntityWorldMut,
    params: Option<&mut DynamicTuple>,
    target: Option<&Entity>,
    component_id: &ComponentId,
    type_id: &TypeId,
    type_registry: &RwLockReadGuard<TypeRegistry>,
    mut closure: Box<dyn ObservableFn>
) -> Option<(Vec<Entity>, bool)> {
    // get the source LazyImmutable component as an ECS change detection handle
    let mut mut_untyped = entity.get_mut_by_id(*component_id).unwrap();

    // ...and convert that into a trait object
    let observable = ph_nglui_mglw_nafh_cthulhu_r_lyeh_wgah_nagl_fhtagn(
        &mut mut_untyped,
        type_id,
        type_registry
    );

    // run the supplied fn
    closure(Box::new(observable), params, target)
}
