use std::{ any::TypeId, sync::RwLockReadGuard };

use bevy::{ ecs::component::ComponentId, prelude::*, reflect::TypeRegistry };

use crate::{ arcane_wizardry::run_as_observable, framework::* };

/// These are the reference user API systems, patterned after the TC39 proposal.
pub mod computed;
pub mod effect;
pub mod init;
pub mod signal;

/// Convenience fn to subscribe an entity to a source.
fn subscribe(
    entity: &Entity,
    source: &Entity,
    type_registry: &RwLockReadGuard<TypeRegistry>,
    world: &mut World
) {
    // get the TypeId of each source (Signal or Memo) Immutable component
    let mut component_id: Option<ComponentId> = None;
    let mut type_id: Option<TypeId> = None;

    trace!("Subscribing {:#?} to {:?}", entity, source);

    // get a readonly reference to the source entity
    if let Some(source) = world.get_entity(*source) {
        trace!("-got source EntityRef");
        // get the source Immutable component
        if let Some(immutable_state) = source.get::<ImmutableState>() {
            trace!("-got ImmutableState");
            // ...as a SignalsObservable
            component_id = Some(immutable_state.component_id);
            if let Some(info) = world.components().get_info(component_id.unwrap()) {
                trace!("-got TypeId");
                type_id = info.type_id();
            }
        }
    }

    // we have a component and a type, now do mutable stuff
    if component_id.is_some() && type_id.is_some() {
        if let Some(mut source) = world.get_entity_mut(*source) {
            let component_id = &component_id.unwrap();
            let type_id = type_id.unwrap();

            run_as_observable(
                &mut source,
                None,
                Some(entity),
                component_id,
                &type_id,
                type_registry,
                Box::new(|observable, _params, target| {
                    observable.subscribe(*target.unwrap());
                    observable.merge_subscribers();
                    None
                })
            );
        }
    }
}
