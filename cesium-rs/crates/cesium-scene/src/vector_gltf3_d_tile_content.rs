//! Ported from `packages/engine/Source/Scene/VectorGltf3DTileContent.js`.

/// Vector glTF 3D Tiles content.
pub struct VectorGltf3DTileContent {
    _private: (),
}

impl VectorGltf3DTileContent {
    /// Creates a new VectorGltf3DTileContent.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for VectorGltf3DTileContent {
    fn default() -> Self { Self::new() }
}
