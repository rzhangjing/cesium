//! Ported from `packages/engine/Source/Scene/GltfIndexBufferLoader.js`.

/// Loads glTF index buffers.
pub struct GltfIndexBufferLoader {
    _private: (),
}

impl GltfIndexBufferLoader {
    /// Creates a new GltfIndexBufferLoader.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for GltfIndexBufferLoader {
    fn default() -> Self { Self::new() }
}
