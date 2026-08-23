//! Ported from `packages/engine/Source/Scene/MetadataSemantic.js`.

/// Metadata semantic.
pub struct MetadataSemantic {
    _private: (),
}

impl MetadataSemantic {
    /// Creates a new MetadataSemantic.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for MetadataSemantic {
    fn default() -> Self { Self::new() }
}
