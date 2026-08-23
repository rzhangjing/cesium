//! Ported from `packages/engine/Source/Scene/MetadataClass.js`.

/// Metadata class.
pub struct MetadataClass {
    _private: (),
}

impl MetadataClass {
    /// Creates a new MetadataClass.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for MetadataClass {
    fn default() -> Self { Self::new() }
}
