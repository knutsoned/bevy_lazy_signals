use bevy::{ ecs::schedule::SystemConfigs, prelude::* };

mod arcane_wizardry;

pub mod api;

pub mod commands;

pub mod framework;
use framework::*;
use lazy_immutable::*;

pub mod systems;
use systems::{
    computed::compute_memos,
    init::init_lazy_signals,
    signal::send_signals,
    effect::{ apply_deferred_effects, check_tasks },
};

pub mod prelude {
    pub use crate::{ api::*, framework::*, systems::*, LazySignalsPlugin };
}

/// Convenience typedefs.
/// (could not get &String to work)
pub type StaticStrRef = &'static str;
pub type LazySignalsBool = LazySignalsState<bool>;
pub type LazySignalsInt = LazySignalsState<u32>;
pub type LazySignalsFloat = LazySignalsState<f64>;
pub type LazySignalsStr = LazySignalsState<StaticStrRef>;
pub type LazySignalsUnit = LazySignalsState<()>; // for triggers, mostly

/// A reference implementation follows. A developer can replace any or all pieces and provide a new
/// plugin if so desired.
///
/// System set used by plugin to run reference implementation.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct LazySignalsSystemSet;

/// Convenience functions to make it easy to run the LazySignals systems when needed.
pub fn lazy_signals_full_systems() -> SystemConfigs {
    (check_tasks, init_lazy_signals, send_signals, compute_memos, apply_deferred_effects).chain()
}

/// This chain omits the effects sending system to allow the developer to
pub fn lazy_signals_flush_systems() -> SystemConfigs {
    (check_tasks, init_lazy_signals, send_signals, compute_memos).chain()
}

/// Plugin to initialize the resource and system schedule.
pub struct LazySignalsPlugin;

impl Plugin for LazySignalsPlugin {
    fn build(&self, app: &mut App) {
        // NOTE: the user application will need to register each custom LazyImmutable<T> for reflection

        // add the systems to process signals, memos, and effects
        app.add_systems(
            PreUpdate, // could be PostUpdate or whatever else (probably not Update)
            // defaults to PreUpdate since it is assumed the UI will process right after Update

            // PostUpdate is a good place to read any events from the main app and send signals
            // for the next tick to handle

            // should be able to call these systems as often as needed between schedules
            // in that case, use lazy_signals_flush_systems() to schedule the needed updates

            // Last, call apply_deferred_effects() at the end so they only fire once per tick
            lazy_signals_full_systems().in_set(LazySignalsSystemSet)
        )
            // custom Immutable types must be manually registered
            .register_type::<LazySignalsBool>()
            .register_type::<LazySignalsInt>()
            .register_type::<LazySignalsFloat>()
            .register_type::<LazySignalsStr>()
            .register_type::<LazySignalsUnit>();
    }
}
