//! Ported from `packages/engine/Source/Scene/CreateWorldImageryAsync.js`.

/// Creates world imagery asynchronously.
///
/// Factory for Cesium's default world imagery tileset.
pub struct CreateWorldImageryAsync {
    /// Whether creation is complete.
    pub complete: bool,
}

impl CreateWorldImageryAsync {
    /// Creates a new CreateWorldImageryAsync.
    pub fn new() -> Self { Self { complete: false } }
}

impl Default for CreateWorldImageryAsync {
    fn default() -> Self { Self::new() }
}
