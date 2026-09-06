//! Ported from `packages/engine/Source/Scene/SpatialNode.js`.

/// Spatial node.
///
/// A node in a spatial index (octree/BVH).
pub struct SpatialNode {
    /// The number of children.
    pub child_count: u32,
    /// Whether the node is a leaf.
    pub is_leaf: bool,
}

impl SpatialNode {
    /// Creates a new SpatialNode.
    pub fn new() -> Self { Self { child_count: 0, is_leaf: true } }
}

impl Default for SpatialNode {
    fn default() -> Self { Self::new() }
}
