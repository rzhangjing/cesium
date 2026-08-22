//! Ported from `packages/engine/Source/Core/GroundPolylineGeometry.js`.
//! NOTE: Requires `PolylinePipeline` for `create_geometry`.

use crate::cartesian3::Cartesian3;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct GroundPolylineGeometry {
    positions: Vec<Cartesian3>,
    width: f64,
    granularity: f64,
}

impl GroundPolylineGeometry {
    pub fn new(positions: Vec<Cartesian3>, width: Option<f64>, granularity: Option<f64>) -> Self {
        Self { positions, width: width.unwrap_or(1.0), granularity: granularity.unwrap_or(0.017453292519943295) }
    }
    // TODO: create_geometry
}
