//! Ported from `packages/engine/Source/Scene/buildVoxelDrawCommands.js`.

/// Builds draw commands for voxel rendering.
pub struct BuildVoxelDrawCommands {
    _private: (),
}

impl BuildVoxelDrawCommands {
    /// Creates a new BuildVoxelDrawCommands.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for BuildVoxelDrawCommands {
    fn default() -> Self { Self::new() }
}
