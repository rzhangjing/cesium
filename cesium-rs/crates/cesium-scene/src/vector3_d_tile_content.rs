//! Ported from `packages/engine/Source/Scene/Vector3DTileContent.js`.

/// Content for vector 3D tiles.
///
/// Manages the features and geometry of a vector tile.
pub struct Vector3DTileContent {
    /// The number of features.
    pub features_length: u32,
    /// Whether the content is ready.
    pub ready: bool,
    /// Whether the content has a batch table.
    pub has_batch_table: bool,
}

impl Vector3DTileContent {
    /// Creates a new Vector3DTileContent.
    pub fn new() -> Self { Self { features_length: 0, ready: false, has_batch_table: false } }

    /// Returns the number of features.
    pub fn features_length(&self) -> u32 { self.features_length }

    /// Returns true if the content is ready.
    pub fn is_ready(&self) -> bool { self.ready }
}

impl Default for Vector3DTileContent {
    fn default() -> Self { Self::new() }
}
