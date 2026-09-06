//! Ported from `packages/engine/Source/Scene/VectorGltf3DTileContent.js`.

/// Content for vector glTF 3D tiles.
///
/// Manages glTF-based vector tile content with feature properties.
pub struct VectorGltf3DTileContent {
    /// The number of features.
    pub features_length: u32,
    /// Whether the content is ready.
    pub ready: bool,
}

impl VectorGltf3DTileContent {
    /// Creates a new VectorGltf3DTileContent.
    pub fn new() -> Self { Self { features_length: 0, ready: false } }

    /// Returns the number of features.
    pub fn features_length(&self) -> u32 { self.features_length }

    /// Returns true if the content is ready.
    pub fn is_ready(&self) -> bool { self.ready }
}

impl Default for VectorGltf3DTileContent {
    fn default() -> Self { Self::new() }
}
