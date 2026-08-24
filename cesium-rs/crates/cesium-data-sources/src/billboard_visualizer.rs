//! Ported from `packages/engine/Source/DataSources/BillboardVisualizer.js`.
//!
//! A visualizer that creates and manages billboard primitives from entity data.

use std::collections::HashMap;

use crate::bounding_sphere_state::BoundingSphereState;
use crate::entity::Entity;
use crate::visualizer::Visualizer;

/// A visualizer that creates billboard primitives from entity data.
///
/// This visualizer creates and manages `BillboardCollection` instances
/// based on entities with `BillboardGraphics`.
///
/// In CesiumJS, `BillboardVisualizer.js` is ~270 lines. It listens for
/// entity-collection `collectionChanged` events, creates/removes billboards
/// in a `BillboardCollection`, and each `update()` synchronises properties.
///
/// DEVIATION: GPU-side `BillboardCollection` is not stored here; the
/// visualizer tracks which entities have billboard graphics and their
/// property hash so that the adapter layer can create real GPU primitives.
pub struct BillboardVisualizer {
    /// Entity IDs currently tracked (entities with billboard graphics).
    entity_ids: Vec<String>,
    /// Per-entity billboard property snapshot (image, scale, color).
    entity_snapshots: HashMap<String, BillboardSnapshot>,
    /// Whether there are pending entity changes to flush.
    pending_changes: bool,
    /// Total number of `update()` calls since creation.
    update_count: u64,
    is_destroyed: bool,
}

/// Simplified snapshot of billboard properties for change detection.
#[derive(Clone, PartialEq)]
struct BillboardSnapshot {
    show: bool,
    image: Option<String>,
    scale: f64,
}

impl BillboardVisualizer {
    /// Creates a new billboard visualizer.
    pub fn new() -> Self {
        Self {
            entity_ids: Vec::new(),
            entity_snapshots: HashMap::new(),
            pending_changes: false,
            update_count: 0,
            is_destroyed: false,
        }
    }

    /// Returns the number of entities currently tracked by this visualizer.
    pub fn entity_count(&self) -> usize {
        self.entity_ids.len()
    }

    /// Returns the total number of `update()` calls.
    pub fn update_count(&self) -> u64 {
        self.update_count
    }

    /// Informs the visualizer that an entity with billboard graphics has
    /// been added or updated.
    ///
    /// In CesiumJS, this is handled via the `collectionChanged` event
    /// listener. Here the caller (typically `DataSourceDisplay`) invokes
    /// it explicitly.
    pub fn on_entity_added_or_updated(&mut self, entity: &Entity) {
        if let Some(ref bb) = entity.billboard {
            let snapshot = BillboardSnapshot {
                show: bb.show,
                image: bb.image.clone(),
                scale: bb.scale,
            };
            let id = entity.id.clone();
            let changed = self
                .entity_snapshots
                .get(&id)
                .map_or(true, |old| *old != snapshot);
            if changed {
                self.entity_snapshots.insert(id.clone(), snapshot);
                if !self.entity_ids.contains(&id) {
                    self.entity_ids.push(id);
                }
                self.pending_changes = true;
            }
        }
    }

    /// Informs the visualizer that an entity has been removed.
    pub fn on_entity_removed(&mut self, entity_id: &str) {
        if self.entity_snapshots.remove(entity_id).is_some() {
            self.entity_ids.retain(|id| id != entity_id);
            self.pending_changes = true;
        }
    }
}

impl Default for BillboardVisualizer {
    fn default() -> Self {
        Self::new()
    }
}

impl Visualizer for BillboardVisualizer {
    fn update(&mut self, _time: f64) -> bool {
        if self.is_destroyed {
            return false;
        }
        self.update_count += 1;
        // Flush pending entity changes to the (abstracted) BillboardCollection.
        // DEVIATION: Actual GPU billboard creation/update deferred to adapter layer.
        self.pending_changes = false;
        true
    }

    fn get_bounding_sphere(&self, entity: &Entity, _result: &mut [f64; 4]) -> BoundingSphereState {
        if self.is_destroyed {
            return BoundingSphereState::Failed;
        }
        if self.entity_snapshots.contains_key(&entity.id) {
            // DEVIATION: Real bounding sphere requires BillboardCollection lookup
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
        self.entity_snapshots.clear();
        self.pending_changes = false;
        self.is_destroyed = true;
    }
}
