//! Ported from `packages/engine/Source/Scene/Model/extensions/gpm/`.

/// Loader for mesh primitive GPM.
pub struct GltfMeshPrimitiveGpmLoader {
    _private: (),
}

impl GltfMeshPrimitiveGpmLoader {
    /// Creates a new GltfMeshPrimitiveGpmLoader.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for GltfMeshPrimitiveGpmLoader {
    fn default() -> Self { Self::new() }
}
