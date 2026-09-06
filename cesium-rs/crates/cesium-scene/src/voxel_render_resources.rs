//! Ported from `packages/engine/Source/Scene/VoxelRenderResources.js`.

/// Voxel render resources.
///
/// Manages GPU resources for voxel volume rendering.
pub struct VoxelRenderResources {
    /// Whether resources are allocated.
    pub allocated: bool,
}

impl VoxelRenderResources {
    /// Creates a new VoxelRenderResources.
    pub fn new() -> Self { Self { allocated: false } }
}

impl Default for VoxelRenderResources {
    fn default() -> Self { Self::new() }
}
