//! Ported from `packages/engine/Source/Scene/GltfJsonLoader.js`.

/// Loads glTF JSON.
pub struct GltfJsonLoader {
    _private: (),
}

impl GltfJsonLoader {
    /// Creates a new GltfJsonLoader.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for GltfJsonLoader {
    fn default() -> Self { Self::new() }
}
