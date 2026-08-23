//! Ported from `packages/engine/Source/DataSources/DynamicGeometryBatch.js`.

/// Batches dynamic (time-varying) geometry instances.
///
/// Dynamic geometry is re-created each frame because the entity's
/// geometry properties may change over time.
pub struct DynamicGeometryBatch {
    is_destroyed: bool,
}

impl DynamicGeometryBatch {
    /// Creates a new dynamic geometry batch.
    pub fn new() -> Self {
        Self { is_destroyed: false }
    }

    /// Adds a geometry updater to the batch.
    pub fn add(&mut self, _updater_id: &str) {}

    /// Removes a geometry updater from the batch.
    pub fn remove(&mut self, _updater_id: &str) {}

    /// Returns whether this batch has been destroyed.
    pub fn is_destroyed(&self) -> bool { self.is_destroyed }

    /// Destroys this batch.
    pub fn destroy(&mut self) { self.is_destroyed = true; }
}

impl Default for DynamicGeometryBatch {
    fn default() -> Self { Self::new() }
}
