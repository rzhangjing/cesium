//! Ported from `packages/engine/Source/Scene/VoxelCylinderShape.js`.

/// A cylinder-shaped voxel volume.
pub struct VoxelCylinderShape {
    _private: (),
}

impl VoxelCylinderShape {
    /// Creates a new VoxelCylinderShape.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for VoxelCylinderShape {
    fn default() -> Self { Self::new() }
}
