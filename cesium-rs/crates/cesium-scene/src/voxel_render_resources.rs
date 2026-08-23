//! Ported from `packages/engine/Source/Scene/VoxelRenderResources.js`.

/// Rendering resources for voxels.
pub struct VoxelRenderResources {
    _private: (),
}

impl VoxelRenderResources {
    /// Creates a new VoxelRenderResources.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for VoxelRenderResources {
    fn default() -> Self { Self::new() }
}
