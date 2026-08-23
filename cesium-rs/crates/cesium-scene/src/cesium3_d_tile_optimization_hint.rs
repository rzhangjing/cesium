//! Ported from `packages/engine/Source/Scene/Cesium3DTileOptimizationHint.js`.

/// A 3D tile optimization hint.
pub struct Cesium3DTileOptimizationHint {
    _private: (),
}

impl Cesium3DTileOptimizationHint {
    /// Creates a new Cesium3DTileOptimizationHint.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for Cesium3DTileOptimizationHint {
    fn default() -> Self { Self::new() }
}
