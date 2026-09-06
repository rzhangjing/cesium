//! Ported from `packages/engine/Source/Scene/ParseStructuralMetadata.js`.

/// Parses structural metadata.
///
/// Extracts EXT_structural_metadata from tile content.
pub struct ParseStructuralMetadata {
    /// Whether parsing is complete.
    pub complete: bool,
}

impl ParseStructuralMetadata {
    /// Creates a new ParseStructuralMetadata.
    pub fn new() -> Self { Self { complete: false } }
}

impl Default for ParseStructuralMetadata {
    fn default() -> Self { Self::new() }
}
