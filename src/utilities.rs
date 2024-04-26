use bevy_ecs::{ prelude::*, storage::SparseSet };

/// ## Utilities
/// Type alias for SparseSet<Entity, ()>
pub type EntitySet = SparseSet<Entity, ()>;

/// Create an empty sparse set for storing Entities by ID
pub fn empty_set() -> EntitySet {
    EntitySet::new()
}
