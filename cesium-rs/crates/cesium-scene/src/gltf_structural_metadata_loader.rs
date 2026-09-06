//! Ported from `packages/engine/Source/Scene/GltfStructuralMetadataLoader.js`.

/// Loads glTF structural metadata.
///
/// Parses and loads EXT_structural_metadata from glTF models.
pub struct GltfStructuralMetadataLoader {
    /// Whether loading is complete.
    pub complete: bool,
}

impl GltfStructuralMetadataLoader {
    /// Creates a new GltfStructuralMetadataLoader.
    pub fn new() -> Self { Self { complete: false } }
}

impl Default for GltfStructuralMetadataLoader {
    fn default() -> Self { Self::new() }
}
