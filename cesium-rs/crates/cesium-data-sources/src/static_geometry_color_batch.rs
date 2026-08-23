//! Ported from `packages/engine/Source/DataSources/StaticGeometryColorBatch.js`.

/// Batches static geometry instances that use color materials.
///
/// Static geometry batches are combined into a single primitive for
/// efficient rendering. Color-based materials are handled separately
/// from texture-based materials.
pub struct StaticGeometryColorBatch {
    /// Whether the batch has been destroyed.
    is_destroyed: bool,
}

impl StaticGeometryColorBatch {
    /// Creates a new static geometry color batch.
    pub fn new() -> Self {
        Self { is_destroyed: false }
    }

    /// Adds a geometry updater to the batch.
    pub fn add(&mut self, _updater_id: &str) {
        // DEVIATION: Requires geometry instance storage and batch merging
    }

    /// Removes a geometry updater from the batch.
    pub fn remove(&mut self, _updater_id: &str) {
        // DEVIATION: Requires geometry instance removal
    }

    /// Returns whether this batch has been destroyed.
    pub fn is_destroyed(&self) -> bool { self.is_destroyed }

    /// Destroys this batch.
    pub fn destroy(&mut self) {
        self.is_destroyed = true;
    }
}

impl Default for StaticGeometryColorBatch {
    fn default() -> Self { Self::new() }
}
