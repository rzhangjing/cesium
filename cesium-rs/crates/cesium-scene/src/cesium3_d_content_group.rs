//! Ported from `packages/engine/Source/Scene/Cesium3DContentGroup.js`.
//!
//! A group of related 3D Tiles content.

/// A group of related 3D Tiles content.
///
/// Used with the 3DTILES_multiple_contents extension to group
/// multiple content entries within a single tile.
/// Mirrors CesiumJS `Cesium3DContentGroup` (179 lines).
pub struct Cesium3DContentGroup {
    /// The group ID.
    pub group_id: Option<String>,
    /// The number of contents in this group.
    pub content_count: i32,
    /// Whether this group has been loaded.
    loaded: bool,
}

impl Cesium3DContentGroup {
    /// Creates a new Cesium3DContentGroup.
    pub fn new() -> Self {
        Self {
            group_id: None,
            content_count: 0,
            loaded: false,
        }
    }

    /// Returns whether this group has been loaded.
    pub fn is_loaded(&self) -> bool {
        self.loaded
    }
}

impl Default for Cesium3DContentGroup {
    fn default() -> Self { Self::new() }
}
