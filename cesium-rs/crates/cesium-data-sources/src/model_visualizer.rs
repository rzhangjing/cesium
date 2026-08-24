//! Ported from `packages/engine/Source/DataSources/ModelVisualizer.js`.
//!
//! A visualizer that creates and manages model primitives from entity data.

use std::collections::HashSet;

use crate::bounding_sphere_state::BoundingSphereState;
use crate::entity::Entity;
use crate::visualizer::Visualizer;

/// A visualizer that creates model primitives from entity data.
///
/// This visualizer creates and manages `Model` instances based on
/// entities with `ModelGraphics`.
///
/// DEVIATION: GPU-side Model loading/rendering is handled by the adapter
/// layer (cesium-scene `Model` primitive). This visualizer tracks which
/// entities have model graphics.
pub struct ModelVisualizer {
    entity_ids: HashSet<String>,
    pending_changes: bool,
    update_count: u64,
    is_destroyed: bool,
}

impl ModelVisualizer {
    /// Creates a new model visualizer.
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

    /// Informs the visualizer that an entity with model graphics has been added or updated.
    pub fn on_entity_added_or_updated(&mut self, entity: &Entity) {
        if entity.model.is_some() && self.entity_ids.insert(entity.id.clone()) {
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

impl Default for ModelVisualizer {
    fn default() -> Self {
        Self::new()
    }
}

impl Visualizer for ModelVisualizer {
    fn update(&mut self, _time: f64) -> bool {
        if self.is_destroyed {
            return false;
        }
        self.update_count += 1;
        // DEVIATION: Actual GPU model loading/update deferred to adapter layer.
        self.pending_changes = false;
        true
    }

    fn get_bounding_sphere(&self, entity: &Entity, _result: &mut [f64; 4]) -> BoundingSphereState {
        if self.is_destroyed {
            return BoundingSphereState::Failed;
        }
        if self.entity_ids.contains(&entity.id) {
            // DEVIATION: Real bounding sphere requires Model primitive lookup
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
