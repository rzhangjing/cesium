//! Ported from `packages/engine/Source/Scene/MetadataTable.js`.

/// Metadata table.
pub struct MetadataTable {
    _private: (),
}

impl MetadataTable {
    /// Creates a new MetadataTable.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for MetadataTable {
    fn default() -> Self { Self::new() }
}
