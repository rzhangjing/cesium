//! Ported from `packages/engine/Source/Scene/BuildVoxelDrawCommands.js`.

/// Builds voxel draw commands.
///
/// Creates draw commands for voxel volume rendering.
pub struct BuildVoxelDrawCommands {
    /// Whether the build is complete.
    pub complete: bool,
}

impl BuildVoxelDrawCommands {
    /// Creates a new BuildVoxelDrawCommands.
    pub fn new() -> Self { Self { complete: false } }
}

impl Default for BuildVoxelDrawCommands {
    fn default() -> Self { Self::new() }
}
