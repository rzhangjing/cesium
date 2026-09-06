//! Ported from `packages/engine/Source/Scene/Tileset3DTileContent.js`.
//!
//! A tile content that points to an external tileset.

/// Tileset 3D Tiles content.
///
/// Represents a tile whose content is a reference to an external tileset
/// (as opposed to model-based content). All numeric properties return 0
/// since the actual content lives in the child tileset.
/// Mirrors CesiumJS `Tileset3DTileContent` (~200 lines).
pub struct Tileset3DTileContent {
    /// The URL of the external tileset resource.
    pub url: String,
    /// Whether this content has been loaded.
    pub ready: bool,
    /// Feature properties dirty flag.
    pub feature_properties_dirty: bool,
}

impl Tileset3DTileContent {
    /// Creates a new `Tileset3DTileContent`.
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            ready: false,
            feature_properties_dirty: false,
        }
    }

    /// Returns the number of features (always 0 for tileset content).
    pub fn features_length(&self) -> u32 {
        0
    }

    /// Returns the number of points (always 0 for tileset content).
    pub fn points_length(&self) -> u32 {
        0
    }

    /// Returns the number of triangles (always 0 for tileset content).
    pub fn triangles_length(&self) -> u32 {
        0
    }

    /// Returns the geometry byte length (always 0 for tileset content).
    pub fn geometry_byte_length(&self) -> u64 {
        0
    }

    /// Returns the textures byte length (always 0 for tileset content).
    pub fn textures_byte_length(&self) -> u64 {
        0
    }

    /// Marks the content as ready.
    pub fn mark_ready(&mut self) {
        self.ready = true;
    }
}

impl Default for Tileset3DTileContent {
    fn default() -> Self { Self::new("") }
}
