//! Ported from `packages/engine/Source/Scene/Cesium3DTileStyleEngine.js`.

/// 3D tile style engine.
pub struct Cesium3DTileStyleEngine {
    _private: (),
}

impl Cesium3DTileStyleEngine {
    /// Creates a new Cesium3DTileStyleEngine.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for Cesium3DTileStyleEngine {
    fn default() -> Self { Self::new() }
}
