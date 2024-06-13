use bevy::{ ecs::schedule::SystemConfigs, prelude::*, reflect::* };

mod arcane_wizardry;

pub mod api;

pub mod commands;

pub mod framework;
use framework::*;
use lazy_immutable::*;

pub mod systems;
use systems::{
    computed::compute_memos,
    init::init_deriveds,
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
pub type LazySignalsTuple = LazySignalsState<DynamicTuple>;
pub type LazySignalsArray = LazySignalsState<DynamicArray>;
pub type LazySignalsList = LazySignalsState<DynamicList>;
pub type LazySignalsMap = LazySignalsState<DynamicMap>;
pub type LazySignalsStruct = LazySignalsState<DynamicStruct>;
pub type LazySignalsTupleStruct = LazySignalsState<DynamicTupleStruct>;
pub type LazySignalsEnum = LazySignalsState<DynamicEnum>;

/// A reference implementation follows. A developer can replace any or all pieces and provide a new
/// plugin if so desired.
///
/// System set used by plugin to run reference implementation.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct LazySignalsSystemSet;

/// Convenience functions to make it easy to run the LazySignals systems when needed.
pub fn lazy_signals_full_systems() -> SystemConfigs {
    (check_tasks, init_deriveds, send_signals, compute_memos, apply_deferred_effects).chain()
}

/// This chain omits the effects sending system to allow the developer to
pub fn lazy_signals_flush_systems() -> SystemConfigs {
    (check_tasks, init_deriveds, send_signals, compute_memos).chain()
}

/// Shared reactive context resource, aka global state.
///
/// This tracks long-running effects across ticks but otherwise should start fresh each cycle.
/// Main purpose is to provide "stack"-like functionality across systems in the processing chain.
///
/// With more robust error handling and components and/or change detection replacing changes,
/// this resource may not even be necessary.
#[derive(Resource)]
pub struct LazySignalsResource {
    // TODO see if changed and errors can be handled without a resource, and get rid of this struct

    /// Tracks which Signals and Memos actually have changed data.
    pub changed: EntitySet,

    /// Tracks errors that occur when things try to run.
    pub errors: ErrorSet,
}

/// This is a singleton that represents the "global state." It is used during internal updates.
impl LazySignalsResource {
    /// Call this at the start of each run to make sure everything is fresh.
    fn init(&mut self) {
        self.changed.clear();
        self.errors.clear();
    }
}

impl Default for LazySignalsResource {
    fn default() -> Self {
        Self {
            changed: empty_set(),
            errors: ErrorSet::new(),
        }
    }
}

/// Plugin to initialize the resource and system schedule.
pub struct LazySignalsPlugin;

impl Plugin for LazySignalsPlugin {
    fn build(&self, app: &mut App) {
        // NOTE: the user application will need to register each custom LazyImmutable<T> for reflection

        // add the systems to process signals, memos, and effects
        app.init_resource::<LazySignalsResource>()
            // custom Immutable types must be manually registered
            .register_type::<LazySignalsBool>()
            .register_type::<LazySignalsInt>()
            .register_type::<LazySignalsFloat>()
            .register_type::<LazySignalsStr>()
            .register_type::<LazySignalsUnit>()
            /* bevy_reflect types don't implement clone...
            .register_type::<LazySignalsArray>()
            .register_type::<LazySignalsList>()
            .register_type::<LazySignalsMap>()
            .register_type::<LazySignalsState>()
            .register_type::<LazySignalsStruct>()
            .register_type::<LazySignalsTuple>()
            .register_type::<LazySignalsTupleStruct>()
            .register_type::<LazySignalsEnum>()
            */
            .add_systems(
                PreUpdate, // could be PostUpdate or whatever else (probably not Update)
                // defaults to PreUpdate since it is assumed the UI will process right after Update

                // PostUpdate is a good place to read any events from the main app and send signals
                // for the next tick to handle

                // should be able to call these systems as often as needed between schedules
                // in that case, use lazy_signals_flush_systems() to schedule the needed updates

                // Last, call apply_deferred_effects() at the end so they only fire once per tick
                lazy_signals_full_systems().in_set(LazySignalsSystemSet)
            );
    }
}
