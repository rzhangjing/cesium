//! Ported from `packages/engine/Source/Scene/parseFeatureMetadataLegacy.js`.

/// Parses legacy feature metadata.
pub struct ParseFeatureMetadataLegacy {
    _private: (),
}

impl ParseFeatureMetadataLegacy {
    /// Creates a new ParseFeatureMetadataLegacy.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ParseFeatureMetadataLegacy {
    fn default() -> Self { Self::new() }
}
