//! Ported from `packages/engine/Source/Scene/Cesium3DTilesetMetadata.js`.

/// Metadata for 3D Tiles tilesets.
pub struct Cesium3DTilesetMetadata {
    _private: (),
}

impl Cesium3DTilesetMetadata {
    /// Creates a new Cesium3DTilesetMetadata.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for Cesium3DTilesetMetadata {
    fn default() -> Self { Self::new() }
}
