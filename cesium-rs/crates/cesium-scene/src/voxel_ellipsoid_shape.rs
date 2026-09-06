//! Ported from `packages/engine/Source/Scene/VoxelEllipsoidShape.js`.

/// Ellipsoid-shaped voxel volume.
///
/// Defines an ellipsoid shape for voxel rendering.
pub struct VoxelEllipsoidShape {
    /// The radii (x, y, z).
    pub radii: (f64, f64, f64),
}

impl VoxelEllipsoidShape {
    /// Creates a new VoxelEllipsoidShape.
    pub fn new() -> Self { Self { radii: (1.0, 1.0, 1.0) } }
}

impl Default for VoxelEllipsoidShape {
    fn default() -> Self { Self::new() }
}
