//! Ported from `packages/engine/Source/Scene/VoxelCell.js`.

/// A single cell in a voxel volume.
pub struct VoxelCell {
    _private: (),
}

impl VoxelCell {
    /// Creates a new VoxelCell.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for VoxelCell {
    fn default() -> Self { Self::new() }
}
