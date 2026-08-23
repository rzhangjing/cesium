//! Ported from `packages/engine/Source/Scene/VoxelMetadataOrder.js`.

/// The ordering of voxel metadata.
pub struct VoxelMetadataOrder {
    _private: (),
}

impl VoxelMetadataOrder {
    /// Creates a new VoxelMetadataOrder.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for VoxelMetadataOrder {
    fn default() -> Self { Self::new() }
}
