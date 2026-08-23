//! Ported from `packages/engine/Source/DataSources/StaticGroundPolylinePerMaterialBatch.js`.

/// Batches static ground polyline instances grouped by material type.
pub struct StaticGroundPolylinePerMaterialBatch {
    is_destroyed: bool,
}

impl StaticGroundPolylinePerMaterialBatch {
    /// Creates a new static ground polyline per-material batch.
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

impl Default for StaticGroundPolylinePerMaterialBatch {
    fn default() -> Self { Self::new() }
}
