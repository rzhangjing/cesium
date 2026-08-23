//! Ported from `packages/engine/Source/Scene/MetadataEntity.js`.

/// Metadata entity.
pub struct MetadataEntity {
    _private: (),
}

impl MetadataEntity {
    /// Creates a new MetadataEntity.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for MetadataEntity {
    fn default() -> Self { Self::new() }
}
