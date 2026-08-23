//! Ported from `packages/engine/Source/Scene/KeyframeNode.js`.

/// A keyframe node.
pub struct KeyframeNode {
    _private: (),
}

impl KeyframeNode {
    /// Creates a new KeyframeNode.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for KeyframeNode {
    fn default() -> Self { Self::new() }
}
