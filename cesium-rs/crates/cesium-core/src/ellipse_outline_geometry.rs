//! Ported from `packages/engine/Source/Core/EllipseOutlineGeometry.js`.

use crate::cartesian3::Cartesian3;
use crate::ellipsoid::Ellipsoid;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct EllipseOutlineGeometry {
    center: Cartesian3,
    semi_major_axis: f64,
    semi_minor_axis: f64,
    ellipsoid: Ellipsoid,
    height: f64,
    extruded_height: f64,
    granularity: f64,
    number_of_vertical_lines: usize,
}

impl EllipseOutlineGeometry {
    pub fn new(center: Cartesian3, semi_major_axis: f64, semi_minor_axis: f64, ellipsoid: Option<Ellipsoid>, height: Option<f64>, extruded_height: Option<f64>, granularity: Option<f64>, number_of_vertical_lines: Option<usize>) -> Self {
        Self { center, semi_major_axis, semi_minor_axis, ellipsoid: ellipsoid.unwrap_or(Ellipsoid::WGS84.clone()), height: height.unwrap_or(0.0), extruded_height: extruded_height.unwrap_or(0.0), granularity: granularity.unwrap_or(0.017453292519943295), number_of_vertical_lines: number_of_vertical_lines.unwrap_or(16) }
    }
    // TODO: create_geometry
}
