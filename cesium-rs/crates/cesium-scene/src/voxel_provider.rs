//! Ported from `packages/engine/Source/Scene/VoxelProvider.js`.

/// A provider for voxel data.
pub struct VoxelProvider {
    _private: (),
}

impl VoxelProvider {
    /// Creates a new VoxelProvider.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for VoxelProvider {
    fn default() -> Self { Self::new() }
}
