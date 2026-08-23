//! Ported from `packages/engine/Source/Scene/I3SNode.js`.

/// An I3S node.
pub struct I3SNode {
    _private: (),
}

impl I3SNode {
    /// Creates a new I3SNode.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for I3SNode {
    fn default() -> Self { Self::new() }
}
