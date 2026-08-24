//! Ported from `packages/engine/Source/DataSources/Cesium3DTilesetVisualizer.js`.
//!
//! A visualizer that creates 3D Tiles primitives from entity data.

use std::collections::HashSet;

use crate::bounding_sphere_state::BoundingSphereState;
use crate::entity::Entity;
use crate::visualizer::Visualizer;

/// A visualizer that creates 3D Tiles primitives from entity data.
///
/// This visualizer creates and manages `Cesium3DTileset` instances
/// based on entities with `Cesium3DTilesetGraphics`.
///
/// DEVIATION: GPU-side `Cesium3DTileset` loading/rendering is handled
/// by the adapter layer (cesium-scene). This visualizer tracks which
/// entities have 3D tileset graphics.
pub struct Cesium3DTilesetVisualizer {
    entity_ids: HashSet<String>,
    pending_changes: bool,
    update_count: u64,
    is_destroyed: bool,
}

impl Cesium3DTilesetVisualizer {
    /// Creates a new 3D Tiles visualizer.
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

    /// Informs the visualizer that an entity with 3D tileset graphics has been added or updated.
    ///
    /// Note: `Entity` does not currently carry a `cesium3d_tileset` field;
    /// this method is wired for future expansion. The visualizer can still
    /// be tested by adding entities via `on_entity_added_or_updated`.
    pub fn on_entity_added_or_updated(&mut self, entity: &Entity) {
        // DEVIATION: Entity lacks a dedicated Cesium3DTilesetGraphics field;
        // check for a property-bag marker or treat all visual entities as candidates.
        if entity.has_visuals() && self.entity_ids.insert(entity.id.clone()) {
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

impl Default for Cesium3DTilesetVisualizer {
    fn default() -> Self {
        Self::new()
    }
}

impl Visualizer for Cesium3DTilesetVisualizer {
    fn update(&mut self, _time: f64) -> bool {
        if self.is_destroyed {
            return false;
        }
        self.update_count += 1;
        // DEVIATION: Actual Cesium3DTileset loading/update deferred to adapter layer.
        self.pending_changes = false;
        true
    }

    fn get_bounding_sphere(&self, entity: &Entity, _result: &mut [f64; 4]) -> BoundingSphereState {
        if self.is_destroyed {
            return BoundingSphereState::Failed;
        }
        if self.entity_ids.contains(&entity.id) {
            // DEVIATION: Real bounding sphere requires Cesium3DTileset lookup
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
