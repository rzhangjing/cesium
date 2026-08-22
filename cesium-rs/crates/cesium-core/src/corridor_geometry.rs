//! Ported from `packages/engine/Source/Core/CorridorGeometry.js`.
//! NOTE: Requires `CorridorGeometryLibrary` for `create_geometry`.

use crate::cartesian3::Cartesian3;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CorridorGeometry {
    positions: Vec<Cartesian3>,
    width: f64,
    corner_type: u32,
    granularity: f64,
}

impl CorridorGeometry {
    pub fn new(positions: Vec<Cartesian3>, width: f64, corner_type: Option<u32>, granularity: Option<f64>) -> Self {
        Self { positions, width, corner_type: corner_type.unwrap_or(0), granularity: granularity.unwrap_or(0.017453292519943295) }
    }
    // TODO: create_geometry
}
