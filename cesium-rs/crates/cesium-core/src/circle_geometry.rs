//! Ported from `packages/engine/Source/Core/CircleGeometry.js`.
//!
//! A description of a circle on the ellipsoid.
//!
//! NOTE: CircleGeometry is a thin wrapper around EllipseGeometry where
//! `semiMajorAxis == semiMinorAxis == radius`. EllipseGeometry has not
//! yet been ported; this module will be completed in a later milestone.

use crate::cartesian3::Cartesian3;
use crate::ellipsoid::Ellipsoid;
use crate::vertex_format::VertexFormat;

/// A description of a circle on the ellipsoid.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CircleGeometry {
    center: Cartesian3,
    radius: f64,
    ellipsoid: Ellipsoid,
    height: f64,
    extruded_height: f64,
    granularity: f64,
    vertex_format: VertexFormat,
    st_rotation: f64,
}

impl CircleGeometry {
    /// Creates a new `CircleGeometry`.
    pub fn new(
        center: Cartesian3,
        radius: f64,
        ellipsoid: Option<Ellipsoid>,
        height: Option<f64>,
        granularity: Option<f64>,
        vertex_format: Option<VertexFormat>,
        extruded_height: Option<f64>,
        st_rotation: Option<f64>,
    ) -> Self {
        Self {
            center,
            radius,
            ellipsoid: ellipsoid.unwrap_or(Ellipsoid::WGS84.clone()),
            height: height.unwrap_or(0.0),
            extruded_height: extruded_height.unwrap_or(0.0),
            granularity: granularity.unwrap_or(0.02),
            vertex_format: vertex_format.unwrap_or_default(),
            st_rotation: st_rotation.unwrap_or(0.0),
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

    /// The ellipsoid.
    pub fn ellipsoid(&self) -> &Ellipsoid {
        &self.ellipsoid
    }

    // TODO: create_geometry — requires EllipseGeometry port
}
