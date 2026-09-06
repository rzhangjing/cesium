//! Ported from `packages/engine/Source/Scene/ProcessVoxelProperties.js`.

/// Processes voxel properties.
///
/// Evaluates and transforms voxel property data.
pub struct ProcessVoxelProperties {
    /// Whether processing is complete.
    pub complete: bool,
}

impl ProcessVoxelProperties {
    /// Creates a new ProcessVoxelProperties.
    pub fn new() -> Self { Self { complete: false } }
}

impl Default for ProcessVoxelProperties {
    fn default() -> Self { Self::new() }
}
