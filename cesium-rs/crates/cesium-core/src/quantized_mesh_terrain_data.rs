//! Ported from `packages/engine/Source/Core/QuantizedMeshTerrainData.js`.

/// Terrain data in quantized mesh format.
pub struct QuantizedMeshTerrainData {
    _private: (),
}

impl QuantizedMeshTerrainData {
    /// Creates a new QuantizedMeshTerrainData.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for QuantizedMeshTerrainData {
    fn default() -> Self { Self::new() }
}
