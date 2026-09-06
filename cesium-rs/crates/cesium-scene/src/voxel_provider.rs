//! Ported from `packages/engine/Source/Scene/VoxelProvider.js`.

/// Voxel data provider.
///
/// Interface for providing voxel tile data.
pub struct VoxelProvider {
    /// Whether the provider is ready.
    pub ready: bool,
}

impl VoxelProvider {
    /// Creates a new VoxelProvider.
    pub fn new() -> Self { Self { ready: false } }
}

impl Default for VoxelProvider {
    fn default() -> Self { Self::new() }
}
