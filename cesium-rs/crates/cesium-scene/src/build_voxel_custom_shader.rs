//! Ported from `packages/engine/Source/Scene/buildVoxelCustomShader.js`.

/// Builds a custom shader for voxel rendering.
pub struct BuildVoxelCustomShader {
    _private: (),
}

impl BuildVoxelCustomShader {
    /// Creates a new BuildVoxelCustomShader.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for BuildVoxelCustomShader {
    fn default() -> Self { Self::new() }
}
