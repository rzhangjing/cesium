//! Ported from `packages/engine/Source/Scene/Cesium3DTileContentFactory.js`.

/// Factory for creating 3D Tiles content.
pub struct Cesium3DTileContentFactory {
    _private: (),
}

impl Cesium3DTileContentFactory {
    /// Creates a new Cesium3DTileContentFactory.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for Cesium3DTileContentFactory {
    fn default() -> Self { Self::new() }
}
