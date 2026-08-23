//! Ported from `packages/engine/Source/Scene/Cesium3DTilePointFeature.js`.

/// A 3D tile point feature.
pub struct Cesium3DTilePointFeature {
    _private: (),
}

impl Cesium3DTilePointFeature {
    /// Creates a new Cesium3DTilePointFeature.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for Cesium3DTilePointFeature {
    fn default() -> Self { Self::new() }
}
