//! Ported from `packages/engine/Source/Scene/ImplicitSubtreeMetadata.js`.

/// Implicit subtree metadata.
///
/// Metadata associated with an implicit subtree.
pub struct ImplicitSubtreeMetadata {
    /// The number of properties.
    pub property_count: u32,
}

impl ImplicitSubtreeMetadata {
    /// Creates a new ImplicitSubtreeMetadata.
    pub fn new() -> Self { Self { property_count: 0 } }
}

impl Default for ImplicitSubtreeMetadata {
    fn default() -> Self { Self::new() }
}
