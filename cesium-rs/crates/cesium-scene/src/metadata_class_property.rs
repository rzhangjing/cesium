//! Ported from `packages/engine/Source/Scene/MetadataClassProperty.js`.

/// Metadata class property.
pub struct MetadataClassProperty {
    _private: (),
}

impl MetadataClassProperty {
    /// Creates a new MetadataClassProperty.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for MetadataClassProperty {
    fn default() -> Self { Self::new() }
}
