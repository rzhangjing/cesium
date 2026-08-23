//! Ported from `packages/engine/Source/Scene/VoxelPrimitive.js`.
//!
//! A volume-rendered voxel primitive.

use cesium_core::matrix4::Matrix4;

use crate::frame_state::FrameState;

/// A volume-rendered voxel primitive.
///
/// Renders 3D voxel data using ray marching through a shape (box, cylinder, or ellipsoid).
/// Mirrors CesiumJS `VoxelPrimitive` (1252 lines).
pub struct VoxelPrimitive {
    /// Whether this primitive is shown.
    pub show: bool,
    /// The model matrix transforming from voxel space to world space.
    pub model_matrix: Matrix4,
    /// The minimum bounds of the voxel data in local space.
    pub min_bounds: [f64; 3],
    /// The maximum bounds of the voxel data in local space.
    pub max_bounds: [f64; 3],
    /// The number of steps for ray marching.
    pub step_size: f64,
    /// The number of ray march iterations.
    pub max_steps: i32,
    /// The shape type (box, cylinder, or ellipsoid).
    pub shape_type: VoxelShapeType,
    /// Whether the primitive has been destroyed.
    is_destroyed: bool,
}

/// The shape type for a voxel primitive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoxelShapeType {
    /// A box shape.
    Box,
    /// A cylinder shape.
    Cylinder,
    /// An ellipsoid shape.
    Ellipsoid,
}

impl VoxelPrimitive {
    /// Creates a new VoxelPrimitive.
    pub fn new() -> Self {
        Self {
            show: true,
            model_matrix: Matrix4::IDENTITY,
            min_bounds: [-1.0, -1.0, -1.0],
            max_bounds: [1.0, 1.0, 1.0],
            step_size: 1.0,
            max_steps: 1000,
            shape_type: VoxelShapeType::Box,
            is_destroyed: false,
        }
    }

    /// Updates the primitive for the current frame.
    pub fn update(&mut self, _frame_state: &FrameState) {
        // DEVIATION: Requires building ray-marching draw commands and shader uniforms
    }

    /// Returns whether this primitive has been destroyed.
    pub fn is_destroyed(&self) -> bool {
        self.is_destroyed
    }

    /// Destroys this primitive.
    pub fn destroy(&mut self) {
        self.is_destroyed = true;
    }
}

impl Default for VoxelPrimitive {
    fn default() -> Self { Self::new() }
}
