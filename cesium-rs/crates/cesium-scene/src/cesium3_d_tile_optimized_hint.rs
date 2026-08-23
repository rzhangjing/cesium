//! Ported from `packages/engine/Source/Scene/Cesium3DTileOptimizedHint.js`.

/// Optimization hints for 3D Tiles.
pub struct Cesium3DTileOptimizedHint {
    _private: (),
}

impl Cesium3DTileOptimizedHint {
    /// Creates a new Cesium3DTileOptimizedHint.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for Cesium3DTileOptimizedHint {
    fn default() -> Self { Self::new() }
}
