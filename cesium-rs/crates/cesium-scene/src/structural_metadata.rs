//! Ported from `packages/engine/Source/Scene/StructuralMetadata.js`.

/// Structural metadata for 3D Tiles.
pub struct StructuralMetadata {
    _private: (),
}

impl StructuralMetadata {
    /// Creates a new StructuralMetadata.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for StructuralMetadata {
    fn default() -> Self { Self::new() }
}
