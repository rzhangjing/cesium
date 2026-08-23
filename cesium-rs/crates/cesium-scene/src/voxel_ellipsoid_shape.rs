//! Ported from `packages/engine/Source/Scene/VoxelEllipsoidShape.js`.

/// An ellipsoid-shaped voxel volume.
pub struct VoxelEllipsoidShape {
    _private: (),
}

impl VoxelEllipsoidShape {
    /// Creates a new VoxelEllipsoidShape.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for VoxelEllipsoidShape {
    fn default() -> Self { Self::new() }
}
