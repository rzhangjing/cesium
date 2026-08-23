//! Ported from `packages/engine/Source/Scene/GltfBufferViewLoader.js`.

/// Loads glTF buffer views.
pub struct GltfBufferViewLoader {
    _private: (),
}

impl GltfBufferViewLoader {
    /// Creates a new GltfBufferViewLoader.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for GltfBufferViewLoader {
    fn default() -> Self { Self::new() }
}
