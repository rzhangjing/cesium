//! Ported from `packages/engine/Source/Scene/MetadataEnumValue.js`.

/// Metadata enum value.
pub struct MetadataEnumValue {
    _private: (),
}

impl MetadataEnumValue {
    /// Creates a new MetadataEnumValue.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for MetadataEnumValue {
    fn default() -> Self { Self::new() }
}
