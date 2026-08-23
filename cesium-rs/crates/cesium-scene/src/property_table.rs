//! Ported from `packages/engine/Source/Scene/PropertyTable.js`.

/// A property table in structured metadata.
pub struct PropertyTable {
    _private: (),
}

impl PropertyTable {
    /// Creates a new PropertyTable.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for PropertyTable {
    fn default() -> Self { Self::new() }
}
