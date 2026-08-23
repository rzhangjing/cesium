//! Ported from `packages/engine/Source/Scene/SpatialNode.js`.

/// A node in a spatial data structure.
pub struct SpatialNode {
    _private: (),
}

impl SpatialNode {
    /// Creates a new SpatialNode.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for SpatialNode {
    fn default() -> Self { Self::new() }
}
