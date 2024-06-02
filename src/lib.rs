use bevy::{ ecs::schedule::SystemConfigs, prelude::* };

mod arcane_wizardry;

pub mod commands;

pub mod api;

pub mod systems;
use systems::{
    init::{ init_effects, init_memos },
    signal::send_signals,
    effect::apply_deferred_effects,
    LazySignalsResource,
};

pub mod framework;
use framework::*;

pub mod prelude {
    pub use crate::{ api::*, framework::*, systems::*, LazySignalsPlugin };
}

/// A reference implementation follows. A consumer can replace any or all pieces and provide a new plugin.
///
/// System set used by plugin to run reference implementation.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct LazySignalsSystemSet;

/// Convenience function to make it easy to run the LazySignals systems when needed.
pub fn lazy_signals_default_systems() -> SystemConfigs {
    (init_effects, init_memos, send_signals, apply_deferred_effects).chain()
}

/// Plugin to initialize the resource and system schedule.
pub struct LazySignalsPlugin;

impl Plugin for LazySignalsPlugin {
    fn build(&self, app: &mut App) {
        // NOTE: the user application will need to register each custom LazyImmutable<T> for reflection

        // add the systems to process signals, memos, and effects
        app.init_resource::<LazySignalsResource>()
            // custom Immutable types must be manually registered
            .register_type::<LazyImmutableBool>()
            .register_type::<LazyImmutableInt>()
            .register_type::<LazyImmutableFloat>()
            .register_type::<LazyImmutableStr>()
            .register_type::<LazyImmutableUnit>()
            .add_systems(
                PreUpdate, // could be PostUpdate or whatever else (probably not Update)
                // defaults to PreUpdate since it is assumed the UI will process right after Update

                // PostUpdate is a good place to read any events from the main app and send signals
                // for the next tick to handle

                // should be able to call these systems as often as needed between schedules
                lazy_signals_default_systems().in_set(LazySignalsSystemSet)
            );
    }
}
