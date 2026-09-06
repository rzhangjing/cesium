//! Ported from `packages/engine/Source/Scene/ParseFeatureMetadataLegacy.js`.

/// Parses legacy feature metadata.
///
/// Extracts feature metadata from legacy batch table formats.
pub struct ParseFeatureMetadataLegacy {
    /// Whether parsing is complete.
    pub complete: bool,
}

impl ParseFeatureMetadataLegacy {
    /// Creates a new ParseFeatureMetadataLegacy.
    pub fn new() -> Self { Self { complete: false } }
}

impl Default for ParseFeatureMetadataLegacy {
    fn default() -> Self { Self::new() }
}
