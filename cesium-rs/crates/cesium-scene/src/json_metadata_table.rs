//! Ported from `packages/engine/Source/Scene/JsonMetadataTable.js`.

/// JSON metadata table.
pub struct JsonMetadataTable {
    _private: (),
}

impl JsonMetadataTable {
    /// Creates a new JsonMetadataTable.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for JsonMetadataTable {
    fn default() -> Self { Self::new() }
}
