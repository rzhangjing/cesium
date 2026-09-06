//! Ported from `packages/engine/Source/Scene/I3SNode.js`.

/// An I3S node.
///
/// Represents a single node in an I3S scene tree.
pub struct I3SNode {
    /// The node index.
    pub index: u64,
    /// The node level in the tree.
    pub level: u32,
    /// Whether the node is loaded.
    pub loaded: bool,
}

impl I3SNode {
    /// Creates a new I3SNode.
    pub fn new() -> Self { Self { index: 0, level: 0, loaded: false } }
}

impl Default for I3SNode {
    fn default() -> Self { Self::new() }
}
