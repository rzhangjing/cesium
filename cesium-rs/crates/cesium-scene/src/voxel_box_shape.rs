//! Ported from `packages/engine/Source/Scene/VoxelBoxShape.js`.

/// Box-shaped voxel volume.
///
/// Defines an axis-aligned box shape for voxel rendering.
pub struct VoxelBoxShape {
    /// The minimum corner.
    pub min: (f64, f64, f64),
    /// The maximum corner.
    pub max: (f64, f64, f64),
}

impl VoxelBoxShape {
    /// Creates a new VoxelBoxShape.
    pub fn new() -> Self { Self { min: (0.0, 0.0, 0.0), max: (1.0, 1.0, 1.0) } }
}

impl Default for VoxelBoxShape {
    fn default() -> Self { Self::new() }
}
