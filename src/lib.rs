use bevy::{ ecs::schedule::SystemConfigs, prelude::* };

mod arcane_wizardry;

pub mod commands;

pub mod factory;

pub mod reference_impl;
use reference_impl::*;

pub mod api;
use api::*;

pub mod prelude {
    pub use crate::{ api::*, factory::*, LazySignalsPlugin, LazySignalsResource };
}

/// A reference implementation follows. A consumer can replace any or all pieces and provide a new plugin.
///
///Convenience typedefs.
pub type LazySignalsStr = &'static str;
pub type ImmutableBool = LazyImmutable<bool>;
pub type ImmutableInt = LazyImmutable<u32>;
pub type ImmutableFloat = LazyImmutable<f64>;
pub type ImmutableStr = LazyImmutable<LazySignalsStr>;
pub type ImmutableUnit = LazyImmutable<()>;

/// System set used by plugin to run reference implementation.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct LazySignalsSystemSet;

pub fn lazy_signals_system_chain() -> SystemConfigs {
    (init_effects, init_memos, send_signals, apply_deferred_effects).chain()
}

/// Plugin to initialize the resource and system schedule.
pub struct LazySignalsPlugin;

impl Plugin for LazySignalsPlugin {
    fn build(&self, app: &mut App) {
        // NOTE: the user application will need to register each custom Immutable<T> for reflection

        // add the systems to process signals, memos, and effects
        app.init_resource::<LazySignalsResource>()
            // custom Immutable types must be manually registered
            .register_type::<ImmutableBool>()
            .register_type::<ImmutableInt>()
            .register_type::<ImmutableFloat>()
            .register_type::<ImmutableStr>()
            .register_type::<ImmutableUnit>()
            //.register_component_as::<dyn LazyMergeable, LazyImmutable<>>()
            .add_systems(
                PreUpdate, // could be PostUpdate or whatever else (probably not Update)
                // defaults to PreUpdate since it is assumed the UI will process right after Update
                // PostUpdate is a good place to read any events from the main app and send signals
                lazy_signals_system_chain().in_set(LazySignalsSystemSet)
            );
    }
}

/// Shared reactive context resource.
#[derive(Resource)]
pub struct LazySignalsResource {
    /// Tracks triggered entities (Signals to send even if their value did not change).
    pub triggered: EntitySet,

    /// Tracks the currently running iteration (immutable once the iteration starts).
    pub running: EntitySet,

    /// Tracks what will run after the end of the current iteration.
    pub next_running: EntitySet,

    /// Tracks which memos have already been added to a running set.
    pub processed: EntitySet,

    /// Tracks which Signals and Memos actually have changed data.
    pub changed: EntitySet,

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
        self.changed.clear();
        self.deferred.clear();
        // self.effects.clear(); // don't clear this, need.. to remember... what is going on
        self.errors.clear();
    }

    // if there is anext_running set, move it into the running set and empty it
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
            changed: empty_set(),
            deferred: empty_set(),
            effects: empty_set(),
            errors: ErrorSet::new(),
        }
    }
}
