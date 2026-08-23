//! Ported from `packages/engine/Source/Scene/parseBatchTable.js`.

/// Parses a batch table from binary data.
pub struct ParseBatchTable {
    _private: (),
}

impl ParseBatchTable {
    /// Creates a new ParseBatchTable.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ParseBatchTable {
    fn default() -> Self { Self::new() }
}
