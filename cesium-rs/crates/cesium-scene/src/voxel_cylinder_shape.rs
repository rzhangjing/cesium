//! Ported from `packages/engine/Source/Scene/VoxelCylinderShape.js`.

/// Cylinder-shaped voxel volume.
///
/// Defines a cylinder shape for voxel rendering.
pub struct VoxelCylinderShape {
    /// The cylinder radius.
    pub radius: f64,
    /// The cylinder height.
    pub height: f64,
}

impl VoxelCylinderShape {
    /// Creates a new VoxelCylinderShape.
    pub fn new() -> Self { Self { radius: 1.0, height: 1.0 } }
}

impl Default for VoxelCylinderShape {
    fn default() -> Self { Self::new() }
}
