//! Ported from `packages/engine/Source/Scene/MetadataSchema.js`.

/// Metadata schema.
pub struct MetadataSchema {
    _private: (),
}

impl MetadataSchema {
    /// Creates a new MetadataSchema.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for MetadataSchema {
    fn default() -> Self { Self::new() }
}
