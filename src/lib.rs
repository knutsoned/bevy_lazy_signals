use bevy::{ ecs::schedule::SystemConfigs, prelude::* };

mod arcane_wizardry;

pub mod commands;

pub mod api;

pub mod systems;
use systems::{
    init::{ init_effects, init_memos },
    signal::send_signals,
    effect::apply_deferred_effects,
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

/// Shared reactive context resource, aka global state.
/// This tracks long-running effects across ticks but otherwise should start fresh each cycle.
/// Main purpose is to provide "stack"-like functionality across systems in the processing chain.
#[derive(Resource)]
pub struct LazySignalsResource {
    /// Tracks triggered entities (notify subscribers even if the value did not change).
    pub triggered: EntitySet,

    /// Tracks the currently running iteration (immutable once the iteration starts).
    /// Used during signal sending.
    pub running: EntitySet,

    /// Tracks what will run after the end of the current iteration.
    /// Used during signal sending.
    pub next_running: EntitySet,

    /// Tracks which memos have already been added to a running set.
    /// Used during signal sending.
    pub processed: EntitySet,

    /// Tracks the currently running computation.
    pub compute_stack: Vec<Entity>,

    /// Tracks which Signals and Memos actually have changed data.
    pub changed: EntitySet,

    /// Tracks which Signals and Memos might have changed data.
    pub dirty: EntitySet,

    /// Tracks Effects to evaluate for processing.
    pub deferred: EntitySet,

    /// Tracks Effects that are still running and should not be re-triggered.
    pub effects: EntitySet,

    /// Tracks errors that occur when things try to run.
    pub errors: ErrorSet,
}

/// This is a singleton that represents the "global state." It is used during internal updates.
impl LazySignalsResource {
    /// Call this at the start of each run to make sure everything is fresh.
    fn init(&mut self) {
        self.triggered.clear();
        self.running.clear();
        self.next_running.clear();
        self.processed.clear();
        self.compute_stack.clear();
        self.changed.clear();
        self.dirty.clear();
        self.deferred.clear();
        // self.effects.clear(); // don't clear this, need.. to remember... what is going on
        self.errors.clear();
    }

    // if there is a next_running set, move it into the running set and empty it
    pub fn merge_running(&mut self) -> bool {
        if self.next_running.is_empty() {
            false
        } else {
            for index in self.next_running.indices() {
                self.running.insert(index, ());
            }
            self.next_running.clear();
            true
        }
    }
}

impl Default for LazySignalsResource {
    fn default() -> Self {
        Self {
            triggered: empty_set(),
            running: empty_set(),
            next_running: empty_set(),
            processed: empty_set(),
            compute_stack: Vec::new(),
            changed: empty_set(),
            dirty: empty_set(),
            deferred: empty_set(),
            effects: empty_set(),
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
