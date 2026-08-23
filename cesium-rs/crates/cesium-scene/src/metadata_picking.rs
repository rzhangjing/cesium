//! Ported from `packages/engine/Source/Scene/MetadataPicking.js`.

/// Metadata picking utilities.
pub struct MetadataPicking {
    _private: (),
}

impl MetadataPicking {
    /// Creates a new MetadataPicking.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for MetadataPicking {
    fn default() -> Self { Self::new() }
}
