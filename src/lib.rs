use bevy::{
    ecs::{ component::{ ComponentId, ComponentInfo }, schedule::SystemConfigs, storage::SparseSet },
    prelude::*,
};

mod arcane_wizardry;

pub mod commands;

pub mod api;

pub mod systems;
use systems::{
    computed::compute_memos,
    init::{ init_effects, init_memos },
    signal::send_signals,
    effect::apply_deferred_effects,
};

pub mod framework;
use framework::*;
use lazy_immutable::*;

pub mod prelude {
    pub use crate::{ api::*, framework::*, systems::*, LazySignalsPlugin };
}

/// Convenience typedefs.
/// (could not get &String to work)
pub type StaticStrRef = &'static str;
pub type LazyImmutableBool = LazySignalsState<bool>;
pub type LazyImmutableInt = LazySignalsState<u32>;
pub type LazyImmutableFloat = LazySignalsState<f64>;
pub type LazyImmutableStr = LazySignalsState<StaticStrRef>;
pub type LazyImmutableUnit = LazySignalsState<()>; // triggers

/// A reference implementation follows. A consumer can replace any or all pieces and provide a new plugin.
///
/// System set used by plugin to run reference implementation.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct LazySignalsSystemSet;

/// Convenience function to make it easy to run the LazySignals systems when needed.
pub fn lazy_signals_default_systems() -> SystemConfigs {
    (init_effects, init_memos, send_signals, compute_memos, apply_deferred_effects).chain()
}

/// Shared reactive context resource, aka global state.
/// This tracks long-running effects across ticks but otherwise should start fresh each cycle.
/// Main purpose is to provide "stack"-like functionality across systems in the processing chain.
#[derive(Resource)]
pub struct LazySignalsResource {
    /// Tracks which Signals and Memos actually have changed data.
    pub changed: EntitySet,

    /// Tracks which Memos might have changed data.
    pub dirty: EntitySet,

    /// Tracks Effects to evaluate for processing.
    pub deferred: EntitySet,

    /// Tracks Effects that are still running and should not be re-triggered.
    pub long_effects: EntitySet,

    /// Tracks triggered entities (notify subscribers even if the value did not change).
    pub triggered: EntitySet,

    /// Tracks errors that occur when things try to run.
    pub errors: ErrorSet,
}

/// This is a singleton that represents the "global state." It is used during internal updates.
impl LazySignalsResource {
    /// Call this at the start of each run to make sure everything is fresh.
    fn init(&mut self) {
        self.changed.clear();
        self.deferred.clear();
        self.dirty.clear();
        // self.long_effects.clear(); // don't clear this, need.. to remember... what is going on
        self.triggered.clear();
        self.errors.clear();
    }
}

impl Default for LazySignalsResource {
    fn default() -> Self {
        Self {
            changed: empty_set(),
            deferred: empty_set(),
            dirty: empty_set(),
            long_effects: empty_set(),
            triggered: empty_set(),
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

/// ## Utilities
/// Set of Entity to ComponentId.
pub type ComponentIdSet = SparseSet<Entity, ComponentId>;

/// Set of ComponentId to ComponentInfo.
pub type ComponentInfoSet = SparseSet<ComponentId, ComponentInfo>;

/// Set of Entity to child Entities.
pub type EntityRelationshipSet = SparseSet<Entity, Vec<Entity>>;

/// Set of unique Entities
pub type EntitySet = SparseSet<Entity, ()>;

/// Set of internal errors when running computed (propagator) and effect functions.
pub type ErrorSet = SparseSet<Entity, LazySignalsError>;

/// Create an empty sparse set for storing Entities by ID.
pub fn empty_set() -> EntitySet {
    EntitySet::new()
}
