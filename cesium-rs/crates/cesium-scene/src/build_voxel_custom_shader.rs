//! Ported from `packages/engine/Source/Scene/BuildVoxelCustomShader.js`.

/// Builds voxel custom shader.
///
/// Generates custom shader code for voxel rendering.
pub struct BuildVoxelCustomShader {
    /// Whether the build is complete.
    pub complete: bool,
}

impl BuildVoxelCustomShader {
    /// Creates a new BuildVoxelCustomShader.
    pub fn new() -> Self { Self { complete: false } }
}

impl Default for BuildVoxelCustomShader {
    fn default() -> Self { Self::new() }
}
