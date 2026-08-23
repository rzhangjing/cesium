//! Ported from `packages/engine/Source/Scene/MetadataSchemaLoader.js`.

/// Metadata schema loader.
pub struct MetadataSchemaLoader {
    _private: (),
}

impl MetadataSchemaLoader {
    /// Creates a new MetadataSchemaLoader.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for MetadataSchemaLoader {
    fn default() -> Self { Self::new() }
}
