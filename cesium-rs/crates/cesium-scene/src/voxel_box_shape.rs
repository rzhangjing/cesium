//! Ported from `packages/engine/Source/Scene/VoxelBoxShape.js`.

/// A box-shaped voxel volume.
pub struct VoxelBoxShape {
    _private: (),
}

impl VoxelBoxShape {
    /// Creates a new VoxelBoxShape.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for VoxelBoxShape {
    fn default() -> Self { Self::new() }
}
