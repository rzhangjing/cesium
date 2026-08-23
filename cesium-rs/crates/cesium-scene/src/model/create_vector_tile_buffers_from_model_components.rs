//! Ported from `packages/engine/Source/Scene/Model/createVectorTileBuffersFromModelComponents.js`.

/// Creates vector tile buffers from model components.
pub struct CreateVectorTileBuffersFromModelComponents {
    _private: (),
}

impl CreateVectorTileBuffersFromModelComponents {
    /// Creates a new CreateVectorTileBuffersFromModelComponents.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for CreateVectorTileBuffersFromModelComponents {
    fn default() -> Self { Self::new() }
}
