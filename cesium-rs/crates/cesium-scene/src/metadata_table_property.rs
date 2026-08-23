//! Ported from `packages/engine/Source/Scene/MetadataTableProperty.js`.

/// Metadata table property.
pub struct MetadataTableProperty {
    _private: (),
}

impl MetadataTableProperty {
    /// Creates a new MetadataTableProperty.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for MetadataTableProperty {
    fn default() -> Self { Self::new() }
}
