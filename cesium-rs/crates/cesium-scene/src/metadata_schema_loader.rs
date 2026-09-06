//! Ported from `packages/engine/Source/Scene/MetadataSchemaLoader.js`.

/// Metadata schema loader.
///
/// Loads and parses metadata schema definitions.
pub struct MetadataSchemaLoader {
    /// Whether the schema is loaded.
    pub loaded: bool,
}

impl MetadataSchemaLoader {
    /// Creates a new MetadataSchemaLoader.
    pub fn new() -> Self { Self { loaded: false } }
}

impl Default for MetadataSchemaLoader {
    fn default() -> Self { Self::new() }
}
