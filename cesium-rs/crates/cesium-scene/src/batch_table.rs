//! Ported from `packages/engine/Source/Scene/BatchTable.js`.

/// A batch table for per-feature metadata.
pub struct BatchTable {
    _private: (),
}

impl BatchTable {
    /// Creates a new BatchTable.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for BatchTable {
    fn default() -> Self { Self::new() }
}
