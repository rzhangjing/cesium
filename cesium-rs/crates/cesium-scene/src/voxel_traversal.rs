//! Ported from `packages/engine/Source/Scene/VoxelTraversal.js`.

/// Voxel traversal.
///
/// Traverses voxel data for ray marching and volume rendering.
pub struct VoxelTraversal {
    /// The maximum number of steps per ray.
    pub max_steps: u32,
    /// Whether the traversal is ready.
    pub ready: bool,
}

impl VoxelTraversal {
    /// Creates a new VoxelTraversal.
    pub fn new() -> Self { Self { max_steps: 256, ready: false } }
}

impl Default for VoxelTraversal {
    fn default() -> Self { Self::new() }
}
