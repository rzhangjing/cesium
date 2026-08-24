//! Ported from `packages/engine/Source/DataSources/PathVisualizer.js`.
//!
//! A visualizer that creates path primitives from entity data.

use std::collections::HashSet;

use crate::bounding_sphere_state::BoundingSphereState;
use crate::entity::Entity;
use crate::visualizer::Visualizer;

/// A visualizer that creates path primitives from entity data.
///
/// This visualizer creates and manages `PolylineCollection` instances
/// based on entities with `PathGraphics`.
///
/// DEVIATION: GPU-side polyline creation/update deferred to adapter layer.
/// Position sampling for trail geometry is not yet implemented.
pub struct PathVisualizer {
    entity_ids: HashSet<String>,
    pending_changes: bool,
    update_count: u64,
    is_destroyed: bool,
}

impl PathVisualizer {
    /// Creates a new path visualizer.
    pub fn new() -> Self {
        Self {
            entity_ids: HashSet::new(),
            pending_changes: false,
            update_count: 0,
            is_destroyed: false,
        }
    }

    /// Returns the number of entities currently tracked.
    pub fn entity_count(&self) -> usize {
        self.entity_ids.len()
    }

    /// Returns the total number of `update()` calls.
    pub fn update_count(&self) -> u64 {
        self.update_count
    }

    /// Informs the visualizer that an entity with path graphics has been added or updated.
    pub fn on_entity_added_or_updated(&mut self, entity: &Entity) {
        // Entity doesn't currently have a `path` field; check for position
        // presence (path requires a time-dynamic position to trail).
        if entity.position.is_some() && self.entity_ids.insert(entity.id.clone()) {
            self.pending_changes = true;
        }
    }

    /// Informs the visualizer that an entity has been removed.
    pub fn on_entity_removed(&mut self, entity_id: &str) {
        if self.entity_ids.remove(entity_id) {
            self.pending_changes = true;
        }
    }
}

impl Default for PathVisualizer {
    fn default() -> Self {
        Self::new()
    }
}

impl Visualizer for PathVisualizer {
    fn update(&mut self, _time: f64) -> bool {
        if self.is_destroyed {
            return false;
        }
        self.update_count += 1;
        // DEVIATION: Actual GPU polyline creation/position sampling deferred.
        self.pending_changes = false;
        true
    }

    fn get_bounding_sphere(&self, entity: &Entity, _result: &mut [f64; 4]) -> BoundingSphereState {
        if self.is_destroyed {
            return BoundingSphereState::Failed;
        }
        if self.entity_ids.contains(&entity.id) {
            BoundingSphereState::Pending
        } else {
            BoundingSphereState::Failed
        }
    }

    fn is_destroyed(&self) -> bool {
        self.is_destroyed
    }

    fn destroy(&mut self) {
        self.entity_ids.clear();
        self.pending_changes = false;
        self.is_destroyed = true;
    }
}
