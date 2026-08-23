//! Ported from `packages/engine/Source/Scene/GltfVertexBufferLoader.js`.

/// Loads glTF vertex buffers.
pub struct GltfVertexBufferLoader {
    _private: (),
}

impl GltfVertexBufferLoader {
    /// Creates a new GltfVertexBufferLoader.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for GltfVertexBufferLoader {
    fn default() -> Self { Self::new() }
}
