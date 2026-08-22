//! Ported from `packages/engine/Source/Core/CircleOutlineGeometry.js`.
//!
//! A description of the outline of a circle on the ellipsoid.
//!
//! NOTE: CircleOutlineGeometry is a thin wrapper around EllipseOutlineGeometry
//! where `semiMajorAxis == semiMinorAxis == radius`. EllipseOutlineGeometry
//! has not yet been ported; this module will be completed in a later milestone.

use crate::cartesian3::Cartesian3;
use crate::ellipsoid::Ellipsoid;

/// A description of the outline of a circle on the ellipsoid.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CircleOutlineGeometry {
    center: Cartesian3,
    radius: f64,
    ellipsoid: Ellipsoid,
    height: f64,
    extruded_height: f64,
    granularity: f64,
    number_of_vertical_lines: usize,
}

impl CircleOutlineGeometry {
    /// Creates a new `CircleOutlineGeometry`.
    pub fn new(
        center: Cartesian3,
        radius: f64,
        ellipsoid: Option<Ellipsoid>,
        height: Option<f64>,
        granularity: Option<f64>,
        extruded_height: Option<f64>,
        number_of_vertical_lines: Option<usize>,
    ) -> Self {
        Self {
            center,
            radius,
            ellipsoid: ellipsoid.unwrap_or(Ellipsoid::WGS84.clone()),
            height: height.unwrap_or(0.0),
            extruded_height: extruded_height.unwrap_or(0.0),
            granularity: granularity.unwrap_or(0.02),
            number_of_vertical_lines: number_of_vertical_lines.unwrap_or(16),
        }
    }

    /// The circle's center point.
    pub fn center(&self) -> &Cartesian3 {
        &self.center
    }

    /// The circle's radius in meters.
    pub fn radius(&self) -> f64 {
        self.radius
    }

    // TODO: create_geometry — requires EllipseOutlineGeometry port
}
