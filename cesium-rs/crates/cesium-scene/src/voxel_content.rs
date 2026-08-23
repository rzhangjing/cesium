//! Ported from `packages/engine/Source/Scene/VoxelContent.js`.

/// Content of a voxel primitive.
pub struct VoxelContent {
    _private: (),
}

impl VoxelContent {
    /// Creates a new VoxelContent.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for VoxelContent {
    fn default() -> Self { Self::new() }
}
