//! Ported from `packages/engine/Source/Scene/ImplicitSubtree.js`.

/// Implicit subtree.
///
/// Represents a subtree within an implicit tileset.
pub struct ImplicitSubtree {
    /// Whether the subtree is loaded.
    pub loaded: bool,
    /// The subtree level in the tree.
    pub level: u32,
}

impl ImplicitSubtree {
    /// Creates a new ImplicitSubtree.
    pub fn new() -> Self { Self { loaded: false, level: 0 } }
}

impl Default for ImplicitSubtree {
    fn default() -> Self { Self::new() }
}
