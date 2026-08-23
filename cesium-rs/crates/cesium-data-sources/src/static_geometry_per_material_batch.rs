//! Ported from `packages/engine/Source/DataSources/StaticGeometryPerMaterialBatch.js`.

/// Batches static geometry instances grouped by material type.
///
/// Geometry instances with the same material type are batched together
/// for efficient rendering.
pub struct StaticGeometryPerMaterialBatch {
    is_destroyed: bool,
}

impl StaticGeometryPerMaterialBatch {
    /// Creates a new static geometry per-material batch.
    pub fn new() -> Self {
        Self { is_destroyed: false }
    }

    /// Adds a geometry updater to the batch.
    pub fn add(&mut self, _updater_id: &str) {
        // DEVIATION: Requires material grouping and geometry instance storage
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

impl Default for StaticGeometryPerMaterialBatch {
    fn default() -> Self { Self::new() }
}
