//! Ported from `packages/engine/Source/Scene/VoxelShape.js`.

/// Base voxel shape.
///
/// Defines the shape interface for voxel volume rendering.
pub struct VoxelShape {
    /// The shape type identifier.
    pub shape_type: String,
}

impl VoxelShape {
    /// Creates a new VoxelShape.
    pub fn new() -> Self { Self { shape_type: "box".to_string() } }
}

impl Default for VoxelShape {
    fn default() -> Self { Self::new() }
}
