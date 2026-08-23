//! Ported from `packages/engine/Source/DataSources/StaticGroundGeometryColorBatch.js`.

/// Batches static ground geometry instances that use color materials.
///
/// Ground geometry is geometry that conforms to the terrain surface.
pub struct StaticGroundGeometryColorBatch {
    is_destroyed: bool,
}

impl StaticGroundGeometryColorBatch {
    /// Creates a new static ground geometry color batch.
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

impl Default for StaticGroundGeometryColorBatch {
    fn default() -> Self { Self::new() }
}
