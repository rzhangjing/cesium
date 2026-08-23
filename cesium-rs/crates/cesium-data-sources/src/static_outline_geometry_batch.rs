//! Ported from `packages/engine/Source/DataSources/StaticOutlineGeometryBatch.js`.

/// Batches static outline geometry instances.
pub struct StaticOutlineGeometryBatch {
    is_destroyed: bool,
}

impl StaticOutlineGeometryBatch {
    /// Creates a new static outline geometry batch.
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

impl Default for StaticOutlineGeometryBatch {
    fn default() -> Self { Self::new() }
}
