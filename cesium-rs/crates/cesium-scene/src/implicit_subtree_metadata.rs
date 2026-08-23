//! Ported from `packages/engine/Source/Scene/ImplicitSubtreeMetadata.js`.

/// Implicit subtree metadata.
pub struct ImplicitSubtreeMetadata {
    _private: (),
}

impl ImplicitSubtreeMetadata {
    /// Creates a new ImplicitSubtreeMetadata.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ImplicitSubtreeMetadata {
    fn default() -> Self { Self::new() }
}
