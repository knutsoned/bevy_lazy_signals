use bevy::{ ecs::world::World, prelude::*, reflect::DynamicTuple };

use crate::{
    arcane_wizardry::ph_nglui_mglw_nafh_cthulhu_r_lyeh_wgah_nagl_fhtagn,
    empty_set,
    systems::{ add_subs_to_hierarchy, subscribe },
    ComponentIdSet,
    ComponentInfoSet,
    DeferredEffect,
    EntityHierarchySet,
    ImmutableState,
    LazyEffect,
    LazySignalsResource,
};

pub fn apply_deferred_effects(
    world: &mut World,
    query_effects: &mut QueryState<(Entity, &LazyEffect), With<DeferredEffect>>
) {
    trace!("EFFECTS");
    // collapse the query or get world concurrency errors

    // TODO replace with a single set and append both sources and triggers

    // TODO figure out why it's just not subscribing to the triggers
    let mut hierarchy = EntityHierarchySet::new();
    add_subs_to_hierarchy(
        query_effects,
        &mut hierarchy,
        Box::new(|x: &LazyEffect| x.sources.clone()),
        world
    );
    add_subs_to_hierarchy(
        query_effects,
        &mut hierarchy,
        Box::new(|x: &LazyEffect| x.triggers.clone()),
        world
    );

    let mut effects = empty_set();

    trace!("Processing effects {:#?}", hierarchy);

    // read, mostly
    world.resource_scope(|world, mut signals: Mut<LazySignalsResource>| {
        for (effect, sources) in hierarchy.iter() {
            let effect = *effect;
            trace!("Processing effect {:?}", effect);

            // only run an effect if at least one of its sources is in the changed set
            // OR it has been explicitly triggered
            let mut actually_run = false;
            if signals.triggered.contains(effect) {
                trace!("-triggering effect {:#?}", effect);
                actually_run = true;
            } else {
                for source in sources {
                    trace!("-checking changed set for source {:#?}", source);
                    if signals.changed.contains(*source) {
                        trace!("-running effect {:#?} with sources {:#?}", effect, sources);
                        actually_run = true;
                    }
                }
            }
            if actually_run {
                effects.insert(effect, ());
            }

            // add to the processed set
            signals.processed.insert(effect, ());

            // remove the DeferredEffect component
            world.entity_mut(effect).remove::<DeferredEffect>();

            // make sure if effects are deferred but not run that they still refresh
            // otherwise they will not be notified next time
            world.resource_scope(|world, type_registry: Mut<AppTypeRegistry>| {
                let type_registry = type_registry.read();
                for source in sources {
                    subscribe(&effect, source, &type_registry, world);
                }
            });
        }
    });

    // write
    for effect in effects.indices() {
        if let Some(sources) = hierarchy.get(effect) {
            trace!("-found effect with sources {:#?}", sources);

            // add the source component ID to the set (probably could be optimized)
            let mut component_id_set = ComponentIdSet::new();
            let mut component_info = ComponentInfoSet::new();

            // build component id -> info map
            for source in sources.iter() {
                let immutable = world.entity(*source).get::<ImmutableState>().unwrap();
                let component_id = immutable.component_id;
                trace!("-found a source with component ID {:#?}", component_id);
                component_id_set.insert(*source, component_id);
                if let Some(info) = world.components().get_info(component_id) {
                    component_info.insert(component_id, info.clone());
                }
            }

            world.resource_scope(|world, type_registry: Mut<AppTypeRegistry>| {
                let type_registry = type_registry.read();
                // prepare the params
                let mut params = DynamicTuple::default();
                for source in sources.iter() {
                    let component_id = component_id_set.get(*source).unwrap();
                    let type_id = component_info.get(*component_id).unwrap().type_id().unwrap();

                    // call the copy_data method via reflection
                    // this will append the source data to the params tuple
                    // FIXME indicate an error if the params don't line up?
                    if let Some(mut source) = world.get_entity_mut(*source) {
                        // insert arcane wizardry here
                        ph_nglui_mglw_nafh_cthulhu_r_lyeh_wgah_nagl_fhtagn(
                            &mut source,
                            &effect,
                            &mut params,
                            component_id,
                            &type_id,
                            &type_registry
                        );
                    }
                }

                // actually run the effect
                world.resource_scope(|world, mut _signals: Mut<LazySignalsResource>| {
                    let world = world.as_unsafe_world_cell();
                    if let Some(handle) = world.get_entity(effect) {
                        // safety (from the docs):
                        // -the UnsafeEntityCell has permission to access the component mutably
                        // -no other references to the component exist at the same time
                        unsafe {
                            let mut effect = handle.get_mut::<LazyEffect>().unwrap();

                            // I think this world must not be used to mutate the effect, not sure
                            (effect.function)(&params, world.world_mut());
                        }
                    }
                });
            });
        }
    }
}
