//! Ported from `packages/engine/Source/Scene/MetadataType.js`.

/// Metadata type.
pub struct MetadataType {
    _private: (),
}

impl MetadataType {
    /// Creates a new MetadataType.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for MetadataType {
    fn default() -> Self { Self::new() }
}
