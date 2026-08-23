//! Ported from `packages/engine/Source/Scene/MetadataComponentType.js`.

/// Metadata component type.
pub struct MetadataComponentType {
    _private: (),
}

impl MetadataComponentType {
    /// Creates a new MetadataComponentType.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for MetadataComponentType {
    fn default() -> Self { Self::new() }
}
