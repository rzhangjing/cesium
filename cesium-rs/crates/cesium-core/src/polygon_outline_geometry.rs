//! Ported from `packages/engine/Source/Core/PolygonOutlineGeometry.js`.

use crate::cartesian3::Cartesian3;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PolygonOutlineGeometry {
    positions: Vec<Cartesian3>,
    height: f64,
    extruded_height: f64,
    granularity: f64,
}

impl PolygonOutlineGeometry {
    pub fn new(positions: Vec<Cartesian3>, height: Option<f64>, extruded_height: Option<f64>, granularity: Option<f64>) -> Self {
        Self { positions, height: height.unwrap_or(0.0), extruded_height: extruded_height.unwrap_or(0.0), granularity: granularity.unwrap_or(0.017453292519943295) }
    }
    // TODO: create_geometry
}
