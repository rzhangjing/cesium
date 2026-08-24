//! Ported from `packages/engine/Source/DataSources/LabelVisualizer.js`.
//!
//! A visualizer that creates and manages label primitives from entity data.

use std::collections::HashMap;

use crate::bounding_sphere_state::BoundingSphereState;
use crate::entity::Entity;
use crate::visualizer::Visualizer;

/// A visualizer that creates label primitives from entity data.
///
/// This visualizer creates and manages `LabelCollection` instances
/// based on entities with `LabelGraphics`.
///
/// DEVIATION: GPU-side `LabelCollection` is not stored here; the
/// visualizer tracks which entities have label graphics so that the
/// adapter layer can create real GPU primitives.
pub struct LabelVisualizer {
    entity_ids: Vec<String>,
    entity_snapshots: HashMap<String, LabelSnapshot>,
    pending_changes: bool,
    update_count: u64,
    is_destroyed: bool,
}

#[derive(Clone, PartialEq)]
struct LabelSnapshot {
    show: bool,
    text: Option<String>,
    scale: f64,
}

impl LabelVisualizer {
    /// Creates a new label visualizer.
    pub fn new() -> Self {
        Self {
            entity_ids: Vec::new(),
            entity_snapshots: HashMap::new(),
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

    /// Informs the visualizer that an entity with label graphics has been added or updated.
    pub fn on_entity_added_or_updated(&mut self, entity: &Entity) {
        if let Some(ref lbl) = entity.label {
            let snapshot = LabelSnapshot {
                show: lbl.show,
                text: lbl.text.clone(),
                scale: lbl.scale,
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

impl Default for LabelVisualizer {
    fn default() -> Self {
        Self::new()
    }
}

impl Visualizer for LabelVisualizer {
    fn update(&mut self, _time: f64) -> bool {
        if self.is_destroyed {
            return false;
        }
        self.update_count += 1;
        // DEVIATION: Actual GPU label creation/update deferred to adapter layer.
        self.pending_changes = false;
        true
    }

    fn get_bounding_sphere(&self, entity: &Entity, _result: &mut [f64; 4]) -> BoundingSphereState {
        if self.is_destroyed {
            return BoundingSphereState::Failed;
        }
        if self.entity_snapshots.contains_key(&entity.id) {
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
