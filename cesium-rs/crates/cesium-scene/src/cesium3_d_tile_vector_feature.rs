//! Ported from `packages/engine/Source/Scene/Cesium3DTileVectorFeature.js`.

/// A 3D tile vector feature.
pub struct Cesium3DTileVectorFeature {
    _private: (),
}

impl Cesium3DTileVectorFeature {
    /// Creates a new Cesium3DTileVectorFeature.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for Cesium3DTileVectorFeature {
    fn default() -> Self { Self::new() }
}
