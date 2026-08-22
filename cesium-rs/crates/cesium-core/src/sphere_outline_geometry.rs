//! Ported from `packages/engine/Source/Core/SphereOutlineGeometry.js`.
//!
//! A description of the outline of a sphere.
//!
//! NOTE: SphereOutlineGeometry is a thin wrapper around EllipsoidOutlineGeometry
//! where all radii are equal. EllipsoidOutlineGeometry has not yet been ported;
//! this module will be completed in a later milestone.

/// A description of the outline of a sphere.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SphereOutlineGeometry {
    radius: f64,
    stack_partitions: usize,
    slice_partitions: usize,
    subdivisions: usize,
}

impl SphereOutlineGeometry {
    /// Creates a new `SphereOutlineGeometry`.
    pub fn new(
        radius: Option<f64>,
        stack_partitions: Option<usize>,
        slice_partitions: Option<usize>,
        subdivisions: Option<usize>,
    ) -> Self {
        Self {
            radius: radius.unwrap_or(1.0),
            stack_partitions: stack_partitions.unwrap_or(10),
            slice_partitions: slice_partitions.unwrap_or(8),
            subdivisions: subdivisions.unwrap_or(200),
        }
    }

    /// The sphere radius.
    pub fn radius(&self) -> f64 {
        self.radius
    }

    // TODO: create_geometry — requires EllipsoidOutlineGeometry port
}
