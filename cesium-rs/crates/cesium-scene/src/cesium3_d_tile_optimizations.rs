//! Ported from `packages/engine/Source/Scene/Cesium3DTileOptimizations.js`.

/// 3D tile optimizations.
pub struct Cesium3DTileOptimizations {
    _private: (),
}

impl Cesium3DTileOptimizations {
    /// Creates a new Cesium3DTileOptimizations.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for Cesium3DTileOptimizations {
    fn default() -> Self { Self::new() }
}
