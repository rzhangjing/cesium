//! Ported from `packages/engine/Source/Scene/Model/extensions/gpm/`.

/// Loader for glTF GPM extension.
pub struct GltfGpmLoader {
    _private: (),
}

impl GltfGpmLoader {
    /// Creates a new GltfGpmLoader.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for GltfGpmLoader {
    fn default() -> Self { Self::new() }
}
