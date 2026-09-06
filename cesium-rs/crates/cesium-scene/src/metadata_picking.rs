//! Ported from `packages/engine/Source/Scene/MetadataPicking.js`.

/// Metadata picking.
///
/// Provides picking support for structural metadata features.
pub struct MetadataPicking {
    /// Whether metadata picking is active.
    pub active: bool,
}

impl MetadataPicking {
    /// Creates a new MetadataPicking.
    pub fn new() -> Self { Self { active: false } }
}

impl Default for MetadataPicking {
    fn default() -> Self { Self::new() }
}
