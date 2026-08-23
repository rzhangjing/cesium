//! Ported from `packages/engine/Source/Scene/parseStructuralMetadata.js`.

/// Parses structural metadata from 3D Tiles.
pub struct ParseStructuralMetadata {
    _private: (),
}

impl ParseStructuralMetadata {
    /// Creates a new ParseStructuralMetadata.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ParseStructuralMetadata {
    fn default() -> Self { Self::new() }
}
