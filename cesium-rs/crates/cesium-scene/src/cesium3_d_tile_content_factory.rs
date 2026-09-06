//! Ported from `packages/engine/Source/Scene/Cesium3DTileContentFactory.js`.

/// Factory for creating 3D tile content instances.
///
/// Maps content types to their respective loader implementations.
pub struct Cesium3DTileContentFactory {
    /// Map of content type strings to factory identifiers.
    pub type_map: Vec<(String, String)>,
}

impl Cesium3DTileContentFactory {
    /// Creates a new Cesium3DTileContentFactory.
    pub fn new() -> Self { Self { type_map: Vec::new() } }
}

impl Default for Cesium3DTileContentFactory {
    fn default() -> Self { Self::new() }
}
