//! Ported from `packages/engine/Source/Scene/GltfStructuralMetadataLoader.js`.

/// Loads glTF structural metadata.
pub struct GltfStructuralMetadataLoader {
    _private: (),
}

impl GltfStructuralMetadataLoader {
    /// Creates a new GltfStructuralMetadataLoader.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for GltfStructuralMetadataLoader {
    fn default() -> Self { Self::new() }
}
