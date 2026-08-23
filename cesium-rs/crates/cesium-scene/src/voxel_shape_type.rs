//! Ported from `packages/engine/Source/Scene/VoxelShapeType.js`.

/// The type of voxel shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum VoxelShapeType {
    /// Box shape.
    Box = 0,
    /// Cylinder shape.
    Cylinder = 1,
    /// Ellipsoid shape.
    Ellipsoid = 2,
}
