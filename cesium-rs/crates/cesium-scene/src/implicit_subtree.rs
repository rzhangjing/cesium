//! Ported from `packages/engine/Source/Scene/ImplicitSubtree.js`.

/// Implicit subtree.
pub struct ImplicitSubtree {
    _private: (),
}

impl ImplicitSubtree {
    /// Creates a new ImplicitSubtree.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ImplicitSubtree {
    fn default() -> Self { Self::new() }
}
