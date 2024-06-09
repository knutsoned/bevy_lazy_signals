use bevy::{ ecs::world::World, prelude::* };

use crate::{ arcane_wizardry::run_as_observable, framework::* };

pub fn compute_memos(
    world: &mut World,
    query_memos: &mut QueryState<(Entity, &ComputedImmutable), With<ComputeMemo>>
) {
    trace!("MEMOS");

    let stack = Vec::<Entity>::new();
    // run each Propagator function to compute memo, adding it and sources to the stack
    // do not run this Propagator if already in the processed set
    // do not add a source if source already in the processed set

    // if a source is marked dirty, add it to the compute stack

    // main loop: evaluate highest index (pop the stack),
    // evaluate that source as above

    // if all sources are up to date, then recompute

    // *** update the data in the cell

    // add the computed entity to the processed set

    // add to the changed set if the value actually changed

    // remove the ComputeMemo component

    // merge all next_subscribers sets into subscribers(?)
}
