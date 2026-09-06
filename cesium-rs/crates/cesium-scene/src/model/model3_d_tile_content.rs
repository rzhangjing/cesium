//! Ported from `packages/engine/Source/Scene/Model/Model3DTileContent.js`.
//!
//! 3D Tiles content backed by a glTF model (b3dm, i3dm, pnts, glTF, etc.).

use serde_json::Value;

/// 3D Tiles content backed by a model.
///
/// Implements the `Cesium3DTileContent` interface for model-based tiles.
/// Stores references to the owning tileset/tile, the loaded model state,
/// and per-content feature/metadata access.
/// Mirrors CesiumJS `Model3DTileContent` (~500 lines).
pub struct Model3DTileContent {
    /// The URL of the content resource.
    pub url: String,
    /// Whether the content has finished loading and is ready.
    pub ready: bool,
    /// The number of features in this content.
    pub features_length: u32,
    /// The number of points (for pnts content).
    pub points_length: u32,
    /// The number of triangles in the content geometry.
    pub triangles_length: u32,
    /// The geometry data size in bytes.
    pub geometry_byte_length: u64,
    /// The texture data size in bytes.
    pub textures_byte_length: u64,
    /// The batch table data size in bytes.
    pub batch_table_byte_length: u64,
    /// The metadata associated with this content, if any.
    pub metadata: Option<Value>,
    /// The content group this content belongs to, if any.
    pub group: Option<String>,
    /// Feature properties dirty flag.
    pub feature_properties_dirty: bool,
}

impl Model3DTileContent {
    /// Creates a new `Model3DTileContent`.
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            ready: false,
            features_length: 0,
            points_length: 0,
            triangles_length: 0,
            geometry_byte_length: 0,
            textures_byte_length: 0,
            batch_table_byte_length: 0,
            metadata: None,
            group: None,
            feature_properties_dirty: false,
        }
    }

    /// Returns the total byte length of all content data.
    pub fn total_byte_length(&self) -> u64 {
        self.geometry_byte_length
            + self.textures_byte_length
            + self.batch_table_byte_length
    }

    /// Returns whether this content has metadata.
    pub fn has_metadata(&self) -> bool {
        self.metadata.is_some()
    }

    /// Marks the content as ready (loaded and processed).
    pub fn mark_ready(&mut self) {
        self.ready = true;
    }
}

impl Default for Model3DTileContent {
    fn default() -> Self { Self::new("") }
}
