//! Ported from `packages/engine/Source/Scene/GltfImageLoader.js`.

/// Loads glTF images.
pub struct GltfImageLoader {
    _private: (),
}

impl GltfImageLoader {
    /// Creates a new GltfImageLoader.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for GltfImageLoader {
    fn default() -> Self { Self::new() }
}
