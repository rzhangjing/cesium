//! Ported from `packages/engine/Source/Scene/MetadataSchema.js`.

/// Metadata schema.
///
/// Defines the schema for structural metadata classes and properties.
pub struct MetadataSchema {
    /// The schema ID.
    pub id: String,
    /// The schema name.
    pub name: String,
    /// The number of classes.
    pub class_count: u32,
}

impl MetadataSchema {
    /// Creates a new MetadataSchema.
    pub fn new() -> Self { Self { id: String::new(), name: String::new(), class_count: 0 } }
}

impl Default for MetadataSchema {
    fn default() -> Self { Self::new() }
}
