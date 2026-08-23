//! Ported from `packages/engine/Source/Scene/VoxelTraversal.js`.

/// Traversal of a voxel volume.
pub struct VoxelTraversal {
    _private: (),
}

impl VoxelTraversal {
    /// Creates a new VoxelTraversal.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for VoxelTraversal {
    fn default() -> Self { Self::new() }
}
