//! Ported from `packages/engine/Source/Scene/GltfSpzLoader.js`.

/// Loads glTF SPZ data.
pub struct GltfSpzLoader {
    _private: (),
}

impl GltfSpzLoader {
    /// Creates a new GltfSpzLoader.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for GltfSpzLoader {
    fn default() -> Self { Self::new() }
}
