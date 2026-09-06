//! Ported from `packages/engine/Source/Scene/Cesium3DTilesetMetadata.js`.

/// Metadata associated with a 3D tileset.
///
/// Provides access to EXT_structural_metadata and legacy batch table metadata.
pub struct Cesium3DTilesetMetadata {
    /// The schema from the tileset metadata extension.
    pub schema: Option<serde_json::Value>,
    /// The metadata object from the tileset JSON.
    pub metadata: Option<serde_json::Value>,
    /// The number of schema classes.
    pub class_count: u32,
}

impl Cesium3DTilesetMetadata {
    /// Creates a new Cesium3DTilesetMetadata.
    pub fn new() -> Self {
        Self { schema: None, metadata: None, class_count: 0 }
    }

    /// Returns true if the tileset has metadata.
    pub fn has_metadata(&self) -> bool { self.metadata.is_some() }
}

impl Default for Cesium3DTilesetMetadata {
    fn default() -> Self { Self::new() }
}
