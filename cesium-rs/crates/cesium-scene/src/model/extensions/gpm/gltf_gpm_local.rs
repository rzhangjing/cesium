//! Ported from `packages/engine/Source/Scene/Model/extensions/gpm/`.

/// Local GPM data for glTF.
pub struct GltfGpmLocal {
    _private: (),
}

impl GltfGpmLocal {
    /// Creates a new GltfGpmLocal.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for GltfGpmLocal {
    fn default() -> Self { Self::new() }
}
