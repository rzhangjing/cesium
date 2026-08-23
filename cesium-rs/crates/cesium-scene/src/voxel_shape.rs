//! Ported from `packages/engine/Source/Scene/VoxelShape.js`.

/// The shape of a voxel volume.
pub struct VoxelShape {
    _private: (),
}

impl VoxelShape {
    /// Creates a new VoxelShape.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for VoxelShape {
    fn default() -> Self { Self::new() }
}
