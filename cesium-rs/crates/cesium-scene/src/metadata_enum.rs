//! Ported from `packages/engine/Source/Scene/MetadataEnum.js`.

/// Metadata enum.
pub struct MetadataEnum {
    _private: (),
}

impl MetadataEnum {
    /// Creates a new MetadataEnum.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for MetadataEnum {
    fn default() -> Self { Self::new() }
}
