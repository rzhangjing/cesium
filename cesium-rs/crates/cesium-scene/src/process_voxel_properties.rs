//! Ported from `packages/engine/Source/Scene/processVoxelProperties.js`.

/// Processes voxel properties for rendering.
pub struct ProcessVoxelProperties {
    _private: (),
}

impl ProcessVoxelProperties {
    /// Creates a new ProcessVoxelProperties.
    pub fn new() -> Self { Self { _private: () } }
}

impl Default for ProcessVoxelProperties {
    fn default() -> Self { Self::new() }
}
