//! Ported from `packages/engine/Source/Core/EllipsoidOutlineGeometry.js`.

use crate::cartesian3::Cartesian3;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct EllipsoidOutlineGeometry {
    radii: Cartesian3,
    inner_radii: Option<Cartesian3>,
    stack_partitions: usize,
    slice_partitions: usize,
    subdivisions: usize,
}

impl EllipsoidOutlineGeometry {
    pub fn new(radii: Option<Cartesian3>, inner_radii: Option<Cartesian3>, stack_partitions: Option<usize>, slice_partitions: Option<usize>, subdivisions: Option<usize>) -> Self {
        Self { radii: radii.unwrap_or(Cartesian3::new(1.0, 1.0, 1.0)), inner_radii, stack_partitions: stack_partitions.unwrap_or(10), slice_partitions: slice_partitions.unwrap_or(8), subdivisions: subdivisions.unwrap_or(200) }
    }
    // TODO: create_geometry
}
