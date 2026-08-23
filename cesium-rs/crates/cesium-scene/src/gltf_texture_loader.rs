//! Ported from `packages/engine/Source/Scene/GltfTextureLoader.js`.

/// Loads glTF textures.
pub struct GltfTextureLoader {
    _private: (),
}

impl GltfTextureLoader {
    /// Creates a new GltfTextureLoader.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for GltfTextureLoader {
    fn default() -> Self { Self::new() }
}
