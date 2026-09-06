//! Ported from `packages/engine/Source/Scene/Model/ModelImagery.js`.
//!
//! Manages imagery draped over a model's surface, tracking per-primitive
//! imagery mappings and detecting configuration changes.

/// Imagery data for a model.
///
/// Manages the collection of per-primitive imagery mappings and tracks
/// imagery-layer configuration snapshots to detect changes.
/// Mirrors CesiumJS `ModelImagery` (~250 lines).
pub struct ModelImagery {
    /// The number of primitive imageries managed.
    pub primitive_imagery_count: usize,
    /// Whether all imagery layers and primitive imageries are ready.
    pub ready: bool,
    /// Whether the imagery configurations have been modified since last update.
    pub configurations_modified: bool,
    /// Per-primitive imagery ready states.
    pub primitive_ready_states: Vec<bool>,
    /// Imagery configuration snapshots (one per imagery layer).
    pub imagery_configurations: Vec<ImageryLayerSnapshot>,
}

/// A snapshot of an imagery layer's configuration, used to detect changes.
#[derive(Debug, Clone, PartialEq)]
pub struct ImageryLayerSnapshot {
    /// The imagery layer index.
    pub layer_index: usize,
    /// Whether this layer is currently enabled.
    pub enabled: bool,
    /// The imagery layer alpha (opacity).
    pub alpha: f64,
}

impl ModelImagery {
    /// Creates a new `ModelImagery`.
    pub fn new() -> Self {
        Self {
            primitive_imagery_count: 0,
            ready: false,
            configurations_modified: false,
            primitive_ready_states: Vec::new(),
            imagery_configurations: Vec::new(),
        }
    }

    /// Returns whether all primitive imageries are ready.
    pub fn all_primitives_ready(&self) -> bool {
        self.primitive_ready_states.iter().all(|&r| r)
    }

    /// Sets the ready state for a specific primitive.
    pub fn set_primitive_ready(&mut self, index: usize, ready: bool) {
        if let Some(slot) = self.primitive_ready_states.get_mut(index) {
            *slot = ready;
        }
    }

    /// Adds a primitive imagery slot.
    pub fn add_primitive(&mut self) -> usize {
        let idx = self.primitive_imagery_count;
        self.primitive_ready_states.push(false);
        self.primitive_imagery_count += 1;
        idx
    }

    /// Updates the imagery configuration snapshot and returns whether it changed.
    pub fn update_configurations(&mut self, new_configs: Vec<ImageryLayerSnapshot>) -> bool {
        let changed = self.imagery_configurations != new_configs;
        self.imagery_configurations = new_configs;
        self.configurations_modified = changed;
        changed
    }
}

impl Default for ModelImagery {
    fn default() -> Self { Self::new() }
}
