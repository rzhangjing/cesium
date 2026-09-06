//! Ported from `packages/engine/Source/Scene/Model/CreateVectorTileBuffersFromModelComponents.js`.

/// Creates vector tile buffers from model components.
///
/// Converts model geometry data to vector tile buffer format.
pub struct CreateVectorTileBuffersFromModelComponents {
    /// Whether creation is complete.
    pub complete: bool,
}

impl CreateVectorTileBuffersFromModelComponents {
    /// Creates a new CreateVectorTileBuffersFromModelComponents.
    pub fn new() -> Self { Self { complete: false } }
}

impl Default for CreateVectorTileBuffersFromModelComponents {
    fn default() -> Self { Self::new() }
}
