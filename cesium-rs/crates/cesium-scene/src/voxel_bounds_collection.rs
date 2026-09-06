//! Ported from `packages/engine/Source/Scene/VoxelBoundsCollection.js`.

/// Voxel bounds collection.
///
/// Collection of voxel bounding volumes.
pub struct VoxelBoundsCollection {
    /// The number of bounds.
    pub length: u32,
}

impl VoxelBoundsCollection {
    /// Creates a new VoxelBoundsCollection.
    pub fn new() -> Self { Self { length: 0 } }
}

impl Default for VoxelBoundsCollection {
    fn default() -> Self { Self::new() }
}
