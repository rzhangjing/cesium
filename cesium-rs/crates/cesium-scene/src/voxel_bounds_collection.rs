//! Ported from `packages/engine/Source/Scene/VoxelBoundsCollection.js`.

/// A collection of voxel bounds.
pub struct VoxelBoundsCollection {
    _private: (),
}

impl VoxelBoundsCollection {
    /// Creates a new VoxelBoundsCollection.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for VoxelBoundsCollection {
    fn default() -> Self { Self::new() }
}
